// This exact definition is used by both the native adapter and rustc controls.
// The module alias keeps purust's threaded Rc-to-Arc conversion from changing
// this intentionally thread-confined native resource.
use std::rc as thread_rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static CREATED: AtomicUsize = AtomicUsize::new(0);
static DESTROYED: AtomicUsize = AtomicUsize::new(0);

struct NativeOwner {
    cell: thread_rc::Rc<std::cell::RefCell<i64>>,
    thread: std::thread::ThreadId,
}
impl NativeOwner {
    fn new(value: i64) -> Self {
        CREATED.fetch_add(1, Ordering::SeqCst);
        Self { cell: thread_rc::Rc::new(std::cell::RefCell::new(value)), thread: std::thread::current().id() }
    }
    fn read(&self) -> i64 {
        assert_eq!(self.thread, std::thread::current().id());
        *self.cell.borrow()
    }
    fn add(&self, amount: i64) -> i64 {
        assert_eq!(self.thread, std::thread::current().id());
        *self.cell.borrow_mut() += amount;
        self.read()
    }
}
impl Drop for NativeOwner {
    fn drop(&mut self) {
        assert_eq!(self.thread, std::thread::current().id(), "Owner must die on its creation thread");
        assert_eq!(thread_rc::Rc::strong_count(&self.cell), 1);
        DESTROYED.fetch_add(1, Ordering::SeqCst);
    }
}
