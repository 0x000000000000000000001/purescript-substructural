pub fn LinearLab_AsyncResources_Main_done() -> Value {
    Value::Func1(Func1::Static(|_| {
        println!("ASYNC_RESOURCES_OK");
        Value::Unit
    }))
}
