use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;

// OWNER_SOURCE: hook.mjs inlines owner.rs here before compilation.

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
thread_local! {
    static ARENA: RefCell<HashMap<u64, NativeOwner>> = RefCell::new(HashMap::new());
    static SCOPES: RefCell<Vec<u64>> = RefCell::new(Vec::new());
}

#[derive(Clone)]
pub struct NativeHandle { id: u64, thread: std::thread::ThreadId }
#[derive(Debug, PartialEq)]
enum AccessError { WrongThread, Closed, WrongScope }

impl NativeHandle {
    fn with_owner<T>(&self, use_owner: impl FnOnce(&NativeOwner) -> T) -> Result<T, AccessError> {
        if self.thread != std::thread::current().id() { return Err(AccessError::WrongThread); }
        ARENA.with(|arena| {
            let arena = arena.borrow();
            let owner = arena.get(&self.id).ok_or(AccessError::Closed)?;
            if SCOPES.with(|scopes| scopes.borrow().last().copied()) != Some(self.id) {
                return Err(AccessError::WrongScope);
            }
            Ok(use_owner(owner))
        })
    }
    fn read(&self) -> Result<i64, AccessError> { self.with_owner(NativeOwner::read) }
    fn add(&self, amount: i64) -> Result<i64, AccessError> { self.with_owner(|owner| owner.add(amount)) }
}

struct RegionGuard { handle: NativeHandle, _local: std::marker::PhantomData<thread_rc::Rc<()>> }
impl Drop for RegionGuard {
    fn drop(&mut self) {
        assert_eq!(self.handle.thread, std::thread::current().id());
        let owner = ARENA.with(|arena| arena.borrow_mut().remove(&self.handle.id));
        // Drop outside the arena's RefCell borrow, on the owning thread.
        drop(owner);
        SCOPES.with(|scopes| assert_eq!(scopes.borrow_mut().pop(), Some(self.handle.id)));
    }
}

fn with_scope<T>(initial: i64, use_handle: impl FnOnce(NativeHandle) -> T) -> T {
    let id = NEXT_ID.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |id| id.checked_add(1))
        .expect("Owner identifier space exhausted");
    let handle = NativeHandle { id, thread: std::thread::current().id() };
    ARENA.with(|arena| assert!(arena.borrow_mut().insert(id, NativeOwner::new(initial)).is_none()));
    SCOPES.with(|scopes| scopes.borrow_mut().push(id));
    let _guard = RegionGuard { handle: handle.clone(), _local: std::marker::PhantomData };
    use_handle(handle)
}

pub fn LinearLab_ThreadAffinity_Api_rawWithOwner(initial: i64, use_handle: Func1<Rc<NativeHandle>, Value>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        with_scope(initial, |handle| use_handle(Rc::new(handle)).unwrap_func1()(Value::Unit))
    })))
}
pub fn LinearLab_ThreadAffinity_Api_rawRead(handle: Rc<NativeHandle>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| Value::Int(handle.read().expect("thread/scope contract violated")))))
}
pub fn LinearLab_ThreadAffinity_Api_rawAdd(handle: Rc<NativeHandle>, amount: i64) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| Value::Int(handle.add(amount).expect("thread/scope contract violated")))))
}
pub fn verify_basic() {
    assert_eq!(CREATED.load(Ordering::SeqCst), 4);
    assert_eq!(DESTROYED.load(Ordering::SeqCst), 4);
    ARENA.with(|arena| assert!(arena.borrow().is_empty()));
    SCOPES.with(|scopes| assert!(scopes.borrow().is_empty()));
}

pub fn is_closed(handle: &NativeHandle) -> bool { handle.read() == Err(AccessError::Closed) }

// Deliberately bypass the PureScript region API to test the native backstop.
// Rejections here are dynamic guards, not PureScript typing guarantees.
pub fn verify_guards() {
    let stale = with_scope(10, |handle| {
        let worker_handle = handle.clone();
        let other_thread = std::thread::spawn(move || {
            assert_eq!(worker_handle.read(), Err(AccessError::WrongThread));
            assert_eq!(worker_handle.add(100), Err(AccessError::WrongThread));
        });
        other_thread.join().unwrap();
        assert_eq!(handle.read(), Ok(10));
        assert_eq!(handle.add(2), Ok(12));
        handle
    });
    assert_eq!(stale.read(), Err(AccessError::Closed));
    with_scope(20, |new_handle| {
        assert_ne!(new_handle.id, stale.id);
        assert_eq!(stale.add(1), Err(AccessError::Closed));
        assert_eq!(new_handle.read(), Ok(20));
    });
    with_scope(3, |outer| {
        with_scope(9, |inner| {
            assert_eq!(outer.read(), Err(AccessError::WrongScope));
            assert_eq!(inner.read(), Ok(9));
        });
        assert_eq!(outer.add(1), Ok(4));
    });
    let mut interrupted = None;
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        with_scope(5, |handle| {
            interrupted = Some(handle);
            // Model native unwinding without printing a deliberately expected
            // panic diagnostic. The same RegionGuard also handled the actual
            // PureScript Effect.Exception tested by Main above.
            std::panic::resume_unwind(Box::new("expected-native-unwind"));
        });
    }));
    assert!(failure.is_err());
    assert_eq!(interrupted.unwrap().read(), Err(AccessError::Closed));
    let later = with_scope(8, |handle| move || handle.read());
    assert_eq!(later(), Err(AccessError::Closed));
    // Forgetting a transport handle cannot keep the TLS owner alive.
    with_scope(7, |handle| std::mem::forget(handle));
    // A separate OS thread can have its OWN arena, owner and cleanup.
    assert_eq!(std::thread::spawn(|| with_scope(30, |handle| handle.add(1).unwrap())).join().unwrap(), 31);
    assert_eq!(CREATED.load(Ordering::SeqCst), 12);
    assert_eq!(DESTROYED.load(Ordering::SeqCst), 12);
    ARENA.with(|arena| assert!(arena.borrow().is_empty()));
    SCOPES.with(|scopes| assert!(scopes.borrow().is_empty()));
    println!("THREAD_AFFINITY_GUARDS_OK wrong-thread=blocked stale=blocked nested-scope=blocked unwind=cleaned forgotten-handle=cleaned worker-local=ok");
}
