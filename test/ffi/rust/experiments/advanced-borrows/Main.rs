pub fn LinearLab_AdvancedBorrows_Main_check(label: String, expected: i64, actual: i64) -> Value {
    Value::Func1(Func1::Shared(std::rc::Rc::new(move |_| {
        assert_eq!(actual, expected);
        println!("PASS {}={actual}", purust_core::purust_string_to_utf8_lossy(&label));
        Value::Unit
    })))
}
pub fn LinearLab_AdvancedBorrows_Main_verify() -> Value {
    Value::Func1(Func1::Static(|_| {
        Purs_LinearLab_AdvancedBorrows_Api::verify_backstop();
        println!("ADVANCED_BORROWS_OK");
        Value::Unit
    }))
}
