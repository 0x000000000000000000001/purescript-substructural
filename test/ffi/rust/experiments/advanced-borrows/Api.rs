use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

static CREATED: AtomicUsize = AtomicUsize::new(0);
static CLOSED: AtomicUsize = AtomicUsize::new(0);
struct Cell { bytes: Option<Vec<u8>>, readers: BTreeSet<u64>, writers: Vec<u64>, next: u64 }
pub struct NativeBuffer { cell: Arc<Mutex<Cell>> }
#[derive(Clone, Copy, PartialEq)]
enum Kind { Read, Write }
pub struct NativeView { cell: Arc<Mutex<Cell>>, token: u64, kind: Kind, range: Range<usize> }
pub struct NativeSplit { left: Rc<NativeView>, right: Rc<NativeView> }
#[derive(Debug, PartialEq)]
enum AccessError { Closed, Busy, Expired, Suspended, Range, Byte }

impl Cell {
    fn token(&mut self) -> u64 { self.next += 1; self.next }
    fn check(&self, view: &NativeView) -> Result<(), AccessError> {
        if self.bytes.is_none() { return Err(AccessError::Closed); }
        match view.kind {
            Kind::Read if self.readers.contains(&view.token) && self.writers.is_empty() => Ok(()),
            Kind::Write if self.writers.last() == Some(&view.token) => Ok(()),
            Kind::Write if self.writers.contains(&view.token) => Err(AccessError::Suspended),
            _ => Err(AccessError::Expired),
        }
    }
}
impl NativeBuffer {
    fn new(text: &str) -> Self {
        CREATED.fetch_add(1, Ordering::SeqCst);
        Self { cell: Arc::new(Mutex::new(Cell { bytes: Some(text.as_bytes().to_vec()), readers: BTreeSet::new(), writers: Vec::new(), next: 0 })) }
    }
    fn begin(&self, kind: Kind) -> Result<(NativeView, Lease), AccessError> {
        let mut cell = self.cell.lock().unwrap();
        let len = cell.bytes.as_ref().ok_or(AccessError::Closed)?.len();
        if !cell.writers.is_empty() || (kind == Kind::Write && !cell.readers.is_empty()) { return Err(AccessError::Busy); }
        let token = cell.token();
        if kind == Kind::Read { cell.readers.insert(token); } else { cell.writers.push(token); }
        Ok((NativeView { cell: self.cell.clone(), token, kind, range: 0..len }, Lease { cell: self.cell.clone(), token, kind }))
    }
}

// Ordinary native APIs return actual references. Their lifetimes end inside
// the adapter call; only checked owner/range handles are returned to PS.
fn native_tail(bytes: &[u8]) -> &[u8] { &bytes[usize::from(!bytes.is_empty())..] }
fn native_sub_mut(bytes: &mut [u8], range: Range<usize>) -> &mut [u8] { &mut bytes[range] }
fn native_split(bytes: &mut [u8], at: usize) -> (&mut [u8], &mut [u8]) { bytes.split_at_mut(at) }

impl NativeView {
    fn checksum(&self) -> Result<i64, AccessError> {
        let cell = self.cell.lock().unwrap();
        cell.check(self)?;
        let slice = &cell.bytes.as_ref().unwrap()[self.range.clone()];
        Ok(slice.iter().map(|byte| i64::from(*byte)).sum())
    }
    fn tail(&self) -> Result<Self, AccessError> {
        let cell = self.cell.lock().unwrap();
        cell.check(self)?;
        if self.kind != Kind::Read { return Err(AccessError::Busy); }
        let slice = &cell.bytes.as_ref().unwrap()[self.range.clone()];
        let returned = native_tail(slice);
        let start = self.range.end - returned.len();
        assert_eq!(returned.as_ptr(), cell.bytes.as_ref().unwrap()[start..].as_ptr());
        Ok(Self { cell: self.cell.clone(), token: self.token, kind: Kind::Read, range: start..self.range.end })
    }
    fn fill(&self, value: i64) -> Result<(), AccessError> {
        let value = u8::try_from(value).map_err(|_| AccessError::Byte)?;
        let mut cell = self.cell.lock().unwrap();
        cell.check(self)?;
        if self.kind != Kind::Write { return Err(AccessError::Busy); }
        cell.bytes.as_mut().unwrap()[self.range.clone()].fill(value);
        Ok(())
    }
    fn reborrow(&self, start: usize, len: usize) -> Result<(Self, Lease), AccessError> {
        let mut cell = self.cell.lock().unwrap();
        cell.check(self)?;
        if self.kind != Kind::Write { return Err(AccessError::Busy); }
        let end = start.checked_add(len).ok_or(AccessError::Range)?;
        if end > self.range.len() { return Err(AccessError::Range); }
        let native = native_sub_mut(&mut cell.bytes.as_mut().unwrap()[self.range.clone()], start..end);
        let actual_len = native.len();
        let range = self.range.start + start..self.range.start + start + actual_len;
        let token = cell.token();
        cell.writers.push(token);
        Ok((Self { cell: self.cell.clone(), token, kind: Kind::Write, range }, Lease { cell: self.cell.clone(), token, kind: Kind::Write }))
    }
    fn split(&self, at: usize) -> Result<(NativeSplit, Lease), AccessError> {
        let mut cell = self.cell.lock().unwrap();
        cell.check(self)?;
        if self.kind != Kind::Write { return Err(AccessError::Busy); }
        if at > self.range.len() { return Err(AccessError::Range); }
        let (left, right) = native_split(&mut cell.bytes.as_mut().unwrap()[self.range.clone()], at);
        let lengths = (left.len(), right.len());
        let token = cell.token();
        cell.writers.push(token);
        let middle = self.range.start + lengths.0;
        let left = Rc::new(Self { cell: self.cell.clone(), token, kind: Kind::Write, range: self.range.start..middle });
        let right = Rc::new(Self { cell: self.cell.clone(), token, kind: Kind::Write, range: middle..middle + lengths.1 });
        Ok((NativeSplit { left, right }, Lease { cell: self.cell.clone(), token, kind: Kind::Write }))
    }
}
struct Lease { cell: Arc<Mutex<Cell>>, token: u64, kind: Kind }
impl Drop for Lease {
    fn drop(&mut self) {
        let mut cell = self.cell.lock().unwrap();
        match self.kind {
            Kind::Read => { assert!(cell.readers.remove(&self.token)); },
            Kind::Write => { assert_eq!(cell.writers.pop(), Some(self.token)); },
        }
    }
}
struct Scope { cell: Arc<Mutex<Cell>> }
impl Drop for Scope {
    fn drop(&mut self) {
        let mut cell = self.cell.lock().unwrap();
        assert!(cell.readers.is_empty() && cell.writers.is_empty());
        if cell.bytes.take().is_some() { CLOSED.fetch_add(1, Ordering::SeqCst); }
    }
}
fn checked<T>(result: Result<T, AccessError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            // Access helpers released their lock before this unwind begins.
            let message = if error == AccessError::Byte { "byte out of range".to_owned() } else { format!("borrow access: {error:?}") };
            Purs_Effect_Exception::purust_exception_raise(Purs_Effect_Exception::Effect_Exception_error(message))
        }
    }
}
fn run(action: Value) -> Value { action.unwrap_func1()(Value::Unit) }

pub fn LinearLab_AdvancedBorrows_Api_rawWithBuffer(text: String, use_buffer: Func1<Rc<NativeBuffer>, Value>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        let buffer = Rc::new(NativeBuffer::new(&text));
        let _scope = Scope { cell: buffer.cell.clone() };
        run(use_buffer(buffer))
    })))
}
fn with_view(buffer: Rc<NativeBuffer>, kind: Kind, use_view: Func1<Rc<NativeView>, Value>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        let (view, _lease) = checked(buffer.begin(kind));
        run(use_view(Rc::new(view)))
    })))
}
pub fn LinearLab_AdvancedBorrows_Api_rawWithRead(buffer: Rc<NativeBuffer>, use_view: Func1<Rc<NativeView>, Value>) -> Value { with_view(buffer, Kind::Read, use_view) }
pub fn LinearLab_AdvancedBorrows_Api_rawWithWrite(buffer: Rc<NativeBuffer>, use_view: Func1<Rc<NativeView>, Value>) -> Value { with_view(buffer, Kind::Write, use_view) }
pub fn LinearLab_AdvancedBorrows_Api_rawChecksum(view: Rc<NativeView>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| Value::Int(checked(view.checksum())))))
}
pub fn LinearLab_AdvancedBorrows_Api_rawTail(view: Rc<NativeView>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| Value::Class(Rc::new(Rc::new(checked(view.tail())))))))
}
pub fn LinearLab_AdvancedBorrows_Api_rawFill(view: Rc<NativeView>, value: i64) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| { checked(view.fill(value)); Value::Unit })))
}
pub fn LinearLab_AdvancedBorrows_Api_rawReborrow(view: Rc<NativeView>, start: i64, len: i64, use_view: Func1<Rc<NativeView>, Value>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        let start = checked(usize::try_from(start).map_err(|_| AccessError::Range));
        let len = checked(usize::try_from(len).map_err(|_| AccessError::Range));
        let (child, _lease) = checked(view.reborrow(start, len));
        run(use_view(Rc::new(child)))
    })))
}
pub fn LinearLab_AdvancedBorrows_Api_rawWithSplit(view: Rc<NativeView>, at: i64, use_pair: Func1<Rc<NativeSplit>, Value>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        let at = checked(usize::try_from(at).map_err(|_| AccessError::Range));
        let (pair, _lease) = checked(view.split(at));
        run(use_pair(Rc::new(pair)))
    })))
}
pub fn LinearLab_AdvancedBorrows_Api_rawLeft(pair: Rc<NativeSplit>) -> Rc<NativeView> { pair.left.clone() }
pub fn LinearLab_AdvancedBorrows_Api_rawRight(pair: Rc<NativeSplit>) -> Rc<NativeView> { pair.right.clone() }

pub fn verify_backstop() {
    assert_eq!(CREATED.load(Ordering::SeqCst), 4);
    assert_eq!(CLOSED.load(Ordering::SeqCst), 4);
    let buffer = NativeBuffer::new("abcd");
    let scope = Scope { cell: buffer.cell.clone() };
    let (read, lease1) = buffer.begin(Kind::Read).unwrap();
    let (nested, lease2) = buffer.begin(Kind::Read).unwrap();
    assert_eq!(read.checksum(), Ok(394));
    assert_eq!(nested.tail().unwrap().checksum(), Ok(297));
    assert!(matches!(buffer.begin(Kind::Write), Err(AccessError::Busy)));
    drop(lease2);
    assert_eq!(nested.checksum(), Err(AccessError::Expired));
    drop(lease1);
    let (write, writer) = buffer.begin(Kind::Write).unwrap();
    let (child, child_lease) = write.reborrow(1, 2).unwrap();
    assert_eq!(write.fill(65), Err(AccessError::Suspended));
    assert_eq!(write.checksum(), Err(AccessError::Suspended));
    assert!(matches!(buffer.begin(Kind::Read), Err(AccessError::Busy)));
    child.fill(90).unwrap();
    drop(child_lease);
    assert_eq!(child.fill(66), Err(AccessError::Expired));
    assert_eq!(write.checksum(), Ok(377));
    let (split, split_lease) = write.split(2).unwrap();
    assert!(split.left.range.end <= split.right.range.start);
    split.left.fill(65).unwrap(); split.right.fill(66).unwrap();
    drop(split_lease);
    assert_eq!(split.left.fill(67), Err(AccessError::Expired));
    assert_eq!(write.checksum(), Ok(262));
    drop(writer);
    drop(scope);
    assert_eq!(write.checksum(), Err(AccessError::Closed));
    assert_eq!(CLOSED.load(Ordering::SeqCst), 5);
    println!("NATIVE_BACKSTOP_OK stale=blocked parent-suspended=blocked reentry=blocked split=disjoint");
}
