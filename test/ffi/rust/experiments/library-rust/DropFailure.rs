use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

static CREATED: AtomicUsize = AtomicUsize::new(0);
static METHOD_CALLS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
static DESTRUCTORS: Mutex<Vec<usize>> = Mutex::new(Vec::new());

// Real owned native payload: no Clone or Copy implementation.
struct FailureOwner { id: usize }
impl std::ops::Drop for FailureOwner {
    fn drop(&mut self) { DESTRUCTORS.lock().unwrap().push(self.id); }
}
struct Slot { owner: Mutex<Option<FailureOwner>> }
struct ExpectedDropFailure;

fn make_owner(id: usize) -> Value {
    CREATED.fetch_add(1, Ordering::SeqCst);
    Value::ClassShared(Rc::new(Slot { owner: Mutex::new(Some(FailureOwner { id })) }))
}

pub fn LibraryRust_DropFailure_failingDrop() -> Value {
    Value::Func1(Func1::Static(|value| {
        let resource = value.unwrap_class::<Slot>().owner.lock().unwrap().take()
            .expect("Drop method executed twice on one owner");
        let id = resource.id;
        METHOD_CALLS.lock().unwrap().push(id);
        std::mem::drop(resource);
        if id == 1 {
            // Release the first payload before deliberately unwinding. No
            // mutex guard is held, and resume_unwind avoids an expected panic
            // diagnostic on stderr. This is a tagged native exception, not a
            // claim about PureScript Effect.Exception representation.
            assert_eq!(*DESTRUCTORS.lock().unwrap(), vec![1]);
            std::panic::resume_unwind(Box::new(ExpectedDropFailure));
        }
        Value::Unit
    }))
}

pub fn LibraryRust_DropFailure_verifyImpl(drop_array: Value) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        CREATED.store(0, Ordering::SeqCst);
        METHOD_CALLS.lock().unwrap().clear();
        DESTRUCTORS.lock().unwrap().clear();
        let array = Purs_Data_Array_Unique::unique(vec![make_owner(1), make_owner(2), make_owner(3)]);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            drop_array.unwrap_func1()(array);
        }));
        let payload = outcome.expect_err("The first element's Drop must unwind");
        assert!(payload.is::<ExpectedDropFailure>(), "Unexpected exception would invalidate this probe");
        assert_eq!(CREATED.load(Ordering::SeqCst), 3);
        assert_eq!(*METHOD_CALLS.lock().unwrap(), vec![1], "Remaining protocol Drop methods were not called");
        let mut released = DESTRUCTORS.lock().unwrap().clone();
        assert_eq!(released.len(), 3, "Every native owner must be destroyed exactly once");
        released.sort_unstable();
        assert_eq!(released, vec![1, 2, 3]);
        println!("LIBRARY_DROP_FAILURE_OK created=3 protocol_methods=1 native_destructors=3 exception=expected");
        Value::Unit
    })))
}
