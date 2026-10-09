include!("owner.rs");

#[cfg(send_owner)]
fn main() {
    let owner = NativeOwner::new(1);
    std::thread::spawn(move || owner.read()).join().unwrap();
}

#[cfg(sync_owner)]
fn main() {
    fn require_sync<T: Sync>(_: &T) {}
    let owner = NativeOwner::new(1);
    require_sync(&owner);
}

#[cfg(not(any(send_owner, sync_owner)))]
fn main() {
    let owner = NativeOwner::new(10);
    assert_eq!(owner.add(7), 17);
    drop(owner);
    assert_eq!(CREATED.load(Ordering::SeqCst), 1);
    assert_eq!(DESTROYED.load(Ordering::SeqCst), 1);
    println!("RUST_THREAD_LOCAL_OWNER_OK");
}
