#[path = "native-core.rs"] mod native;

#[cfg(require_unpin)]
fn main() {
    fn requires_unpin(_: impl Unpin) {}
    let control = native::Control::new(1);
    control.acquire().unwrap();
    requires_unpin(native::Operation::new(control).unwrap());
}

#[cfg(not(require_unpin))]
fn main() {
    use std::future::Future;
    use std::sync::{Arc, Barrier};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Poll, Wake, Waker};
    struct Notification(AtomicUsize);
    impl Wake for Notification {
        fn wake(self: Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); }
        fn wake_by_ref(self: &Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); }
    }

    let control = native::Control::new(7);
    control.acquire().unwrap();
    let notification = Arc::new(Notification(AtomicUsize::new(0)));
    let waker = Waker::from(notification.clone());
    let mut context = Context::from_waker(&waker);
    let mut operation = Box::pin(native::Operation::new(control.clone()).unwrap());
    assert!(operation.as_mut().poll(&mut context).is_pending());
    assert!(matches!(native::Operation::new(control.clone()), Err("operation already started")));
    assert_eq!(control.release(), Err("native operation still active"));
    assert_eq!(control.snapshot().dropped, 0);
    assert!(control.complete(false));
    assert_eq!(notification.0.load(Ordering::SeqCst), 1);
    assert_eq!(operation.as_mut().poll(&mut context), Poll::Ready(native::Outcome::Success(24)));
    drop(operation);
    assert!(control.deliver(native::Outcome::Success(24)));
    control.release().unwrap();
    assert!(matches!(native::Operation::new(control.clone()), Err("resource already released")));
    assert_eq!(control.release(), Err("resource already released"));
    assert!(!control.deliver(native::Outcome::Success(24)));
    let first = control.snapshot();
    assert_eq!((first.acquired, first.released, first.dropped, first.future_drops), (1, 1, 1, 1));

    let mut completed = 0;
    let mut cancelled = 0;
    // Actual OS threads compete at a Barrier, independent of Aff scheduling.
    for _ in 0..32 {
        let control = native::Control::new(10);
        control.acquire().unwrap();
        let mut operation = Box::pin(native::Operation::new(control.clone()).unwrap());
        assert!(operation.as_mut().poll(&mut context).is_pending());
        let barrier = Arc::new(Barrier::new(3));
        let left = { let c = control.clone(); let b = barrier.clone();
            std::thread::spawn(move || { b.wait(); c.complete(false) }) };
        let right = { let c = control.clone(); let b = barrier.clone();
            std::thread::spawn(move || { b.wait(); c.cancel(); }) };
        barrier.wait();
        left.join().unwrap();
        right.join().unwrap();
        let outcome = match operation.as_mut().poll(&mut context) {
            Poll::Ready(outcome) => outcome,
            Poll::Pending => panic!("both terminal signals completed"),
        };
        match outcome {
            native::Outcome::Success(33) => completed += 1,
            native::Outcome::Cancelled => cancelled += 1,
            other => panic!("unexpected race result: {other:?}"),
        }
        drop(operation);
        // Cancellation precedes callback delivery in this controlled test,
        // regardless of which terminal signal won the operation's decision.
        assert!(!control.deliver(outcome));
        control.release().unwrap();
        let s = control.snapshot();
        assert_eq!((s.released, s.dropped, s.future_drops, s.suppressed), (1, 1, 1, 1));
    }
    assert_eq!(completed + cancelled, 32);
    println!("NATIVE_ASYNC_CONTROLS_OK races=32 completed={completed} cancelled={cancelled}");
}
