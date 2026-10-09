pub fn LibraryRust_Main_check(result: i64) -> Value {
    Value::Func1(Func1::Shared(std::rc::Rc::new(move |_| {
        assert_eq!(result, 42);
        println!("LIBRARY_RUST_OK");
        Value::Unit
    })))
}
