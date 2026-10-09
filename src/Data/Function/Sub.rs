use std::rc::Rc;

type TupleOwner = Rc<Purs_Data_Tuple::Tuple>;
type MakeTuple = Func2<Value, Value, TupleOwner>;
type Project = Func1<TupleOwner, Value>;
macro_rules! arrow { ($f:expr) => { Value::Func1(Func1::Shared(Rc::new($f))) }; }
fn call(f: &Value, value: Value) -> Value { f.unwrap_func1()(value) }
fn pair(make: &MakeTuple, left: Value, right: Value) -> Value { Value::ClassShared(make(left, right)) }
fn owner(value: &Value) -> TupleOwner { value.unwrap_class_shared::<Purs_Data_Tuple::Tuple>() }

pub fn Data_Function_Sub_composeFFI(after: Value, before: Value) -> Value {
    arrow!(move |value| call(&after, call(&before, value)))
}
pub fn Data_Function_Sub_idFFI() -> Value { Value::Func1(Func1::Static(|value| value)) }
pub fn Data_Function_Sub_runSharedFFI(f: Value, value: Value) -> Value { call(&f, value) }
pub fn Data_Function_Sub_liftSharedFFI(f: Func1<Value, Value>) -> Value { Value::Func1(f) }
pub fn Data_Function_Sub_unsafeCloneFFI(make: MakeTuple) -> Value {
    arrow!(move |value: Value| pair(&make, value.clone(), value))
}
pub fn Data_Function_Sub_unsafeDrop() -> Value { Value::Func1(Func1::Static(|_| Value::Unit)) }
pub fn Data_Function_Sub_fstFFI(drop: Value, fst: Project, snd: Project) -> Value {
    arrow!(move |value: Value| {
        let tuple = owner(&value);
        call(&drop, snd(tuple.clone()));
        fst(tuple)
    })
}
pub fn Data_Function_Sub_sndFFI(drop: Value, fst: Project, snd: Project) -> Value {
    arrow!(move |value: Value| {
        let tuple = owner(&value);
        call(&drop, fst(tuple.clone()));
        snd(tuple)
    })
}
pub fn Data_Function_Sub_cloneTupleFFI(make: MakeTuple, fst: Project, snd: Project, clone_a: Value, clone_b: Value) -> Value {
    arrow!(move |value: Value| {
        let tuple = owner(&value);
        let a = owner(&call(&clone_a, fst(tuple.clone())));
        let b = owner(&call(&clone_b, snd(tuple)));
        let left = pair(&make, fst(a.clone()), fst(b.clone()));
        let right = pair(&make, snd(a), snd(b));
        pair(&make, left, right)
    })
}
pub fn Data_Function_Sub_dropTupleFFI(fst: Project, snd: Project, drop_a: Value, drop_b: Value) -> Value {
    arrow!(move |value: Value| {
        let tuple = owner(&value);
        call(&drop_a, fst(tuple.clone()));
        call(&drop_b, snd(tuple));
        Value::Unit
    })
}
pub fn Data_Function_Sub_borrowFFI(make: MakeTuple, f: Value) -> Value {
    arrow!(move |value: Value| {
        let result = call(&f, value.clone());
        pair(&make, value, result)
    })
}
