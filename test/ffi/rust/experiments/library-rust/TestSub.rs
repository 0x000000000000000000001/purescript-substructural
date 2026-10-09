use std::rc::Rc;
use std::sync::Mutex;
static DROPS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
struct Tracked { name: &'static str, dropped: Mutex<bool> }
fn tracked(name: &'static str) -> Value { Value::ClassShared(Rc::new(Tracked { name, dropped: Mutex::new(false) })) }
pub fn Test_Data_Function_Sub_trackedDrop() -> Value {
    Value::Func1(Func1::Static(|value| {
        let resource = value.unwrap_class::<Tracked>();
        let mut dropped = resource.dropped.lock().unwrap();
        assert!(!*dropped, "resource destroyed twice");
        *dropped = true;
        DROPS.lock().unwrap().push(resource.name);
        Value::Unit
    }))
}
pub fn Test_Data_Function_Sub_assertDrops(drop_pair: Value, keep_second: Value, keep_first: Value, make: Func2<Value, Value, Rc<Purs_Data_Tuple::Tuple>>) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        DROPS.lock().unwrap().clear();
        let tuple = |a,b| Value::ClassShared(make(a,b));
        drop_pair.unwrap_func1()(tuple(tracked("left"),tracked("right")));
        assert_eq!(*DROPS.lock().unwrap(), vec!["left","right"]);
        assert_eq!(keep_second.unwrap_func1()(tuple(tracked("discard-first"),Value::Int(17))).unwrap_int(),17);
        assert_eq!(keep_first.unwrap_func1()(tuple(Value::Int(23),tracked("discard-second"))).unwrap_int(),23);
        assert_eq!(*DROPS.lock().unwrap(), vec!["left","right","discard-first","discard-second"]);
        println!("LIBRARY_DROP_METHODS_OK calls=4");
        Value::Unit
    })))
}
