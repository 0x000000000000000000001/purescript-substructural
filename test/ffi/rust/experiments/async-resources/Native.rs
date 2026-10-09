use std::rc::Rc;
use std::sync::Arc;

// Expanded in the temporary FFI source by this suite's hook.
mod native { include!("native-core.rs"); }
pub struct Control { inner: Arc<native::Control> }
pub type Resource = Control;

fn error(message: &str) -> Value { Purs_Effect_Exception::Effect_Exception_error(message.to_owned()) }
fn ensure(result: Result<(), &'static str>) {
    if let Err(message) = result { Purs_Effect_Exception::purust_exception_raise(error(message)); }
}
pub fn LinearLab_AsyncResources_Native_newControl(seed: i64) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        assert!((0..=250).contains(&seed));
        Value::Class(Rc::new(Rc::new(Control { inner: native::Control::new(seed as u8) })))
    })))
}
pub fn LinearLab_AsyncResources_Native_acquireImpl(control: Rc<Control>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        ensure(control.inner.acquire());
        Value::Class(Rc::new(control.clone()))
    })))
}
pub fn LinearLab_AsyncResources_Native_controlOf(resource: Rc<Resource>) -> Rc<Control> { resource }
pub fn LinearLab_AsyncResources_Native_releaseImpl(resource: Rc<Resource>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| { ensure(resource.inner.release()); Value::Unit })))
}
pub fn LinearLab_AsyncResources_Native_startImpl(
    resource: Rc<Resource>, success: purust_core::Func1<i64, Value>, failure: purust_core::Func1<Value, Value>,
) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        let control = resource.inner.clone();
        let operation = match native::Operation::new(control.clone()) {
            Ok(operation) => operation,
            Err(message) => Purs_Effect_Exception::purust_exception_raise(error(message)),
        };
        let success = success.clone();
        let failure = failure.clone();
        Purs_Effect_Aff::purust_aff_spawn_native(async move {
            let outcome = Box::pin(operation).await;
            Ok(Value::Int(match outcome {
                native::Outcome::Success(value) => value,
                native::Outcome::Failed => -1, native::Outcome::Cancelled => -2,
            }))
        }, move |result| {
            let outcome = match result {
                Ok(value) => match value.unwrap_int() {
                    -1 => native::Outcome::Failed, -2 => native::Outcome::Cancelled,
                    value => native::Outcome::Success(value),
                },
                Err(_) => panic!("native task failed unexpectedly"),
            };
            if control.deliver(outcome) {
                match outcome {
                    native::Outcome::Success(value) => { success(value).unwrap_func1()(Value::Unit); },
                    native::Outcome::Failed => { failure(error("native operation failed")).unwrap_func1()(Value::Unit); },
                    native::Outcome::Cancelled => unreachable!(),
                }
            }
        });
        Value::Unit
    })))
}
pub fn LinearLab_AsyncResources_Native_cancelImpl(resource: Rc<Resource>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| { resource.inner.cancel(); Value::Unit })))
}
pub fn LinearLab_AsyncResources_Native_waitImpl(control: Rc<Control>, event: i64, callback: purust_core::Func1<i64, Value>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        let wait = native::Event { control: control.inner.clone(), kind: event as usize };
        let callback = callback.clone();
        Purs_Effect_Aff::purust_aff_spawn_native(async move { wait.await; Ok(Value::Unit) }, move |result| {
            assert!(result.is_ok());
            callback(0).unwrap_func1()(Value::Unit);
        });
        Value::Unit
    })))
}
pub fn LinearLab_AsyncResources_Native_complete(control: Rc<Control>, fail: bool) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| Value::Bool(control.inner.complete(fail)))))
}
pub fn LinearLab_AsyncResources_Native_markContinued(control: Rc<Control>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| { control.inner.continued(); Value::Unit })))
}
pub fn LinearLab_AsyncResources_Native_verify(control: Rc<Control>, mode: i64) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        let s = control.inner.snapshot();
        let operations = if mode == 5 { 0 } else { 1 };
        assert_eq!((s.acquired, s.released, s.dropped, s.future_drops), (1, 1, 1, operations), "{s:?}");
        assert_eq!(s.delivered + s.suppressed, operations, "{s:?}");
        assert_eq!(s.duplicate_callbacks, 0);
        match mode {
            0 | 4 => assert_eq!((s.delivered, s.suppressed, s.cancel_requests, s.continued), (1, 0, 0, 1)),
            1 => assert_eq!((s.delivered, s.suppressed, s.cancel_requests, s.continued), (1, 0, 0, 0)),
            2 => assert_eq!((s.delivered, s.suppressed, s.cancel_requests, s.continued), (0, 1, 1, 0)),
            3 => {
                assert!(s.cancel_requests <= 1);
                assert!(s.continued <= 1);
                if s.suppressed == 1 { assert_eq!(s.continued, 0, "suppressed completion cannot continue"); }
            },
            5 => assert_eq!((s.delivered, s.suppressed, s.cancel_requests, s.continued), (0, 0, 0, 0)),
            _ => panic!("unknown verification mode"),
        }
        if operations == 1 {
            assert!(!control.inner.deliver(native::Outcome::Success(999)), "duplicate callback refused");
            assert_eq!(control.inner.snapshot().duplicate_callbacks, 1);
        }
        println!("ASYNC_CASE_OK mode={mode} {s:?}");
        Value::Unit
    })))
}
