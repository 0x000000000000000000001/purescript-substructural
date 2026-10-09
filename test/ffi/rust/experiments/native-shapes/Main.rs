pub fn LinearLab_NativeShapes_Main_assertCounts(created: i64, dropped: i64, read: i64) -> Value {
    Value::Func1(Func1::Shared(std::rc::Rc::new(move |_| {
        assert_eq!(Purs_LinearLab_NativeShapes_Cursor::counts(), (created as usize, dropped as usize, read as usize));
        println!("COUNTS created={created} dropped={dropped} read={read}");
        Value::Unit
    })))
}
pub fn LinearLab_NativeShapes_Main_done() -> Value {
    Value::Func1(Func1::Static(|_| { println!("NATIVE_SHAPES_OK"); Value::Unit }))
}

pub fn LinearLab_NativeShapes_Main_pass(label: String) -> Value {
    Value::Func1(Func1::Shared(std::rc::Rc::new(move |_| {
        println!("PASS {}", purust_core::purust_string_to_utf8_lossy(&label));
        Value::Unit
    })))
}
