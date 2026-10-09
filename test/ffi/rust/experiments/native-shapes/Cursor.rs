use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::str::FromStr;
use std::num::ParseIntError;

static CREATED: AtomicUsize = AtomicUsize::new(0);
static DROPPED: AtomicUsize = AtomicUsize::new(0);
static READ: AtomicUsize = AtomicUsize::new(0);

// A real non-Clone resource owned by an iterator closure. The &mut self method
// ensures the closure captures all of Words, including its Drop implementation.
struct Words { remaining: std::vec::IntoIter<std::string::String> }
impl Words {
    fn new(input: &str) -> Self {
        CREATED.fetch_add(1, Ordering::SeqCst);
        let words: Vec<_> = if input.is_empty() { Vec::new() }
            else { input.split(',').map(|word| word.trim().to_owned()).collect() };
        Self { remaining: words.into_iter() }
    }
    fn next_word(&mut self) -> Option<std::string::String> {
        let word = self.remaining.next();
        if word.is_some() { READ.fetch_add(1, Ordering::SeqCst); }
        word
    }
}
impl Drop for Words {
    fn drop(&mut self) { DROPPED.fetch_add(1, Ordering::SeqCst); }
}

#[derive(Debug)]
enum CursorError { Closed, Parse { word: std::string::String, error: ParseIntError } }
impl std::fmt::Display for CursorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => f.write_str("cursor closed"),
            Self::Parse { word, error } => write!(f, "invalid item '{word}': {error}"),
        }
    }
}

fn parse<T: FromStr>(word: &str) -> Result<T, T::Err> { word.parse::<T>() }

// Real impl Iterator return type, with two monomorphizations below. The wide
// parser uses i32 so every value fits PureScript Int; i64 is the native FFI ABI.
// T and the
// iterator's concrete type are erased behind a trait object, not exposed to PS.
fn parsed<T>(input: &str) -> impl Iterator<Item = Result<i64, CursorError>> + Send
where T: FromStr<Err = ParseIntError> + Into<i64> {
    let mut words = Words::new(input);
    std::iter::from_fn(move || words.next_word().map(|word| {
        parse::<T>(&word).map(Into::into).map_err(|error| CursorError::Parse { word, error })
    }))
}
trait CursorOps: Send {
    fn pull(&mut self) -> Result<Option<i64>, CursorError>;
}
struct IteratorCursor<I> { iterator: I }
impl<I> CursorOps for IteratorCursor<I>
where I: Iterator<Item = Result<i64, CursorError>> + Send {
    fn pull(&mut self) -> Result<Option<i64>, CursorError> { self.iterator.next().transpose() }
}
fn boxed<I>(iterator: I) -> Box<dyn CursorOps + Send>
where I: Iterator<Item = Result<i64, CursorError>> + Send + 'static {
    Box::new(IteratorCursor { iterator })
}

pub struct Cursor { iterator: Mutex<Option<Box<dyn CursorOps + Send>>> }
impl Cursor {
    fn pull(&self) -> Result<Option<i64>, CursorError> {
        let (result, release) = {
            let mut slot = self.iterator.lock().unwrap();
            let result = match slot.as_mut() {
                Some(cursor) => cursor.pull(),
                None => Err(CursorError::Closed),
            };
            let release = if matches!(result, Ok(Some(_))) { None } else { slot.take() };
            (result, release)
        };
        // Actual trait-object/iterator/resource destruction occurs outside lock,
        // before translating the Result into a PureScript constructor.
        drop(release);
        result
    }
    fn close(&self) { let release = self.iterator.lock().unwrap().take(); drop(release); }
}
fn cursor_value(iterator: Box<dyn CursorOps + Send>) -> Value {
    Value::Class(Rc::new(Rc::new(Cursor { iterator: Mutex::new(Some(iterator)) })))
}
pub fn LinearLab_NativeShapes_Cursor_wide(input: String) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| cursor_value(boxed(parsed::<i32>(&input))))))
}
pub fn LinearLab_NativeShapes_Cursor_small(input: String) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| cursor_value(boxed(parsed::<u8>(&input))))))
}
pub fn LinearLab_NativeShapes_Cursor_rawNext(on_error: Func1<String, Value>, on_done: Value, on_item: Func1<i64, Value>, cursor: Rc<Cursor>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| match cursor.pull() {
        Ok(Some(value)) => on_item(value),
        Ok(None) => on_done.clone(),
        Err(error) => on_error(error.to_string()),
    })))
}
pub fn LinearLab_NativeShapes_Cursor_close(cursor: Rc<Cursor>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| { cursor.close(); Value::Unit })))
}
pub fn counts() -> (usize, usize, usize) {
    (CREATED.load(Ordering::SeqCst), DROPPED.load(Ordering::SeqCst), READ.load(Ordering::SeqCst))
}
