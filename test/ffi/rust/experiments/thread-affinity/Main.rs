pub fn LinearLab_ThreadAffinity_Main_assertExpired(handle: Value) -> Value {
    Value::Func1(Func1::Shared(std::rc::Rc::new(move |_| {
        let handle = handle.unwrap_class_shared::<Purs_LinearLab_ThreadAffinity_Api::NativeHandle>();
        Value::Bool(Purs_LinearLab_ThreadAffinity_Api::is_closed(&handle))
    })))
}

pub fn LinearLab_ThreadAffinity_Main_verify(first: i64, second: i64, caught: bool, expired: bool) -> Value {
    Value::Func1(Func1::Shared(std::rc::Rc::new(move |_| {
        assert_eq!((first, second), (34, 34));
        assert!(caught, "PureScript exception must be caught outside the owner scope");
        assert!(expired, "An existentially retained PS handle must be closed at runtime");
        Purs_LinearLab_ThreadAffinity_Api::verify_basic();
        Purs_LinearLab_ThreadAffinity_Api::verify_guards();
        println!("THREAD_AFFINITY_OK results=34,34 ps-exception=cleaned ps-existential=expired owners=12 destroyed=12");
        Value::Unit
    })))
}
