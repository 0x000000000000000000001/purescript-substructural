use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize,Ordering};
static CREATED: AtomicUsize = AtomicUsize::new(0);
static DROPPED: AtomicUsize = AtomicUsize::new(0);
static READS: AtomicUsize = AtomicUsize::new(0);
// The owned native resource itself implements neither Clone nor Copy.
struct NativeOwner { bytes: Vec<u8> }
impl std::ops::Drop for NativeOwner {
    fn drop(&mut self) { DROPPED.fetch_add(1,Ordering::SeqCst); }
}
struct Slot { owner: Mutex<Option<NativeOwner>> }
pub fn LibraryRust_Owned_make() -> Value {
    Value::Func1(Func1::Static(|count| {
        CREATED.fetch_add(1,Ordering::SeqCst);
        Value::ClassShared(Rc::new(Slot { owner:Mutex::new(Some(NativeOwner { bytes:vec![7;count.unwrap_int() as usize] })) }))
    }))
}
pub fn LibraryRust_Owned_readSize() -> Value {
    Value::Func1(Func1::Static(|value| {
        let slot = value.unwrap_class::<Slot>();
        let owner = slot.owner.lock().unwrap();
        let owner = owner.as_ref().expect("borrow after owner destruction");
        READS.fetch_add(1,Ordering::SeqCst);
        Value::Int(owner.bytes.len() as i64)
    }))
}
pub fn LibraryRust_Owned_destroy() -> Value {
    Value::Func1(Func1::Static(|value| {
        let owner = value.unwrap_class::<Slot>().owner.lock().unwrap().take().expect("owner destroyed twice");
        drop(owner);
        Value::Unit
    }))
}
pub fn LibraryRust_Owned_verify() -> Value {
    Value::Func1(Func1::Static(|_| {
        assert_eq!(CREATED.load(Ordering::SeqCst),3);
        assert_eq!(DROPPED.load(Ordering::SeqCst),3);
        assert_eq!(READS.load(Ordering::SeqCst),1);
        println!("LIBRARY_OWNED_CONTAINERS_OK created=3 dropped=3 reads=1");
        Value::Unit
    }))
}
