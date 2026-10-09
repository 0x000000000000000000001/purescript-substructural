use std::rc::Rc;
type Project = Func1<Rc<Purs_Data_Tuple::Tuple>, Value>;
pub fn Test_Data_Array_Unique_assertCloneIndependence(clone: Value, reverse: Value, fst: Project, snd: Project) -> Value {
    Value::Func1(Func1::Shared(Rc::new(move |_| {
        let original = Purs_Data_Array_Unique::unique(vec![Value::Int(1),Value::Int(2),Value::Int(3)]);
        let pair = clone.unwrap_func1()(original).unwrap_class_shared::<Purs_Data_Tuple::Tuple>();
        let left = fst(pair.clone());
        let right = snd(pair);
        assert!(!Rc::ptr_eq(&left.unwrap_class_shared::<Purs_Data_Array_Unique::NativeUnique>(), &right.unwrap_class_shared::<Purs_Data_Array_Unique::NativeUnique>()));
        let reversed = reverse.unwrap_func1()(left);
        let to_shared = Purs_Data_Array_Unique::Data_Array_Unique_toSharedFFI().unwrap_func1();
        let values = |v: Value| to_shared(v).unwrap_array().iter().map(Value::unwrap_int).collect::<Vec<_>>();
        assert_eq!(values(reversed), vec![3,2,1]);
        assert_eq!(values(right), vec![1,2,3]);
        let empties = clone.unwrap_func1()(Purs_Data_Array_Unique::unique(Vec::new())).unwrap_class_shared::<Purs_Data_Tuple::Tuple>();
        let a = fst(empties.clone()).unwrap_class_shared::<Purs_Data_Array_Unique::NativeUnique>();
        let b = snd(empties).unwrap_class_shared::<Purs_Data_Array_Unique::NativeUnique>();
        assert!(!Rc::ptr_eq(&a, &b));
        println!("LIBRARY_UNIQUE_CLONE_OK independent=true empty=true");
        Value::Unit
    })))
}
