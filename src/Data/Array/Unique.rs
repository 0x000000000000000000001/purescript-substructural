use std::rc::Rc;
use std::sync::Mutex;
type TupleOwner = Rc<Purs_Data_Tuple::Tuple>;
type MakeTuple = Func2<Value, Value, TupleOwner>;
type Project = Func1<TupleOwner, Value>;
macro_rules! arrow { ($f:expr) => { Value::Func1(Func1::Shared(Rc::new($f))) }; }

// References to this wrapper may be cloned by the backend, but its buffer is
// moved out once. Every consuming operation invalidates the previous wrapper.
pub struct NativeUnique { values: Mutex<Option<Vec<Value>>> }
pub fn unique(values: Vec<Value>) -> Value {
    Value::ClassShared(Rc::new(NativeUnique { values: Mutex::new(Some(values)) }))
}
fn take(value: &Value) -> Vec<Value> {
    value.unwrap_class::<NativeUnique>().values.lock().unwrap().take().expect("unique array already consumed")
}
pub fn Data_Array_Unique_empty() -> Value { arrow!(|_| unique(Vec::new())) }
pub fn Data_Array_Unique_singleton() -> Value { arrow!(|value| unique(vec![value])) }
pub fn Data_Array_Unique_fromSharedFFI() -> Value {
    arrow!(|value: Value| unique(value.unwrap_array().as_ref().clone()))
}
pub fn Data_Array_Unique_toSharedFFI() -> Value { arrow!(|value: Value| mk_array(take(&value))) }
pub fn Data_Array_Unique_reverse() -> Value {
    arrow!(|value: Value| { let mut values = take(&value); values.reverse(); unique(values) })
}
pub fn Data_Array_Unique_length() -> Value {
    arrow!(|value: Value| {
        let array = value.unwrap_class::<NativeUnique>();
        let values = array.values.lock().unwrap();
        Value::Int(values.as_ref().expect("borrow of consumed unique array").len() as i64)
    })
}
pub fn Data_Array_Unique_snocFFI(fst: Project, snd: Project) -> Value {
    arrow!(move |value: Value| {
        let tuple = value.unwrap_class_shared::<Purs_Data_Tuple::Tuple>();
        let mut values = take(&fst(tuple.clone()));
        values.push(snd(tuple));
        unique(values)
    })
}
pub fn Data_Array_Unique_cloneUniqueArrayFFI(clone: Value, make: MakeTuple, fst: Project, snd: Project) -> Value {
    arrow!(move |value: Value| {
        let mut left = Vec::new();
        let mut right = Vec::new();
        for element in take(&value) {
            let copies = clone.unwrap_func1()(element).unwrap_class_shared::<Purs_Data_Tuple::Tuple>();
            left.push(fst(copies.clone()));
            right.push(snd(copies));
        }
        Value::ClassShared(make(unique(left), unique(right)))
    })
}
pub fn Data_Array_Unique_dropUniqueArrayFFI(drop: Value) -> Value {
    arrow!(move |value: Value| {
        for element in take(&value) { drop.unwrap_func1()(element); }
        Value::Unit
    })
}
