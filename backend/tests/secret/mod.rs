use super::*;

#[test]
fn the_memory_store_keeps_and_forgets_the_key() {
    let memory = Memory::default();
    assert_eq!(memory.load(), Ok(None));
    memory.save("key").unwrap();
    assert_eq!(memory.load(), Ok(Some("key".to_owned())));
    memory.forget().unwrap();
    assert_eq!(memory.load(), Ok(None));
}

#[test]
#[ignore = "writes a test entry in the protected storage of the session"]
fn real_keyring_keeps_and_forgets_a_test_entry() {
    let entry = keyring::Entry::new("nas-sync-client-test", "probe").unwrap();
    entry.set_password("probe").unwrap();
    assert_eq!(entry.get_password().unwrap(), "probe");
    entry.delete_credential().unwrap();
    assert!(matches!(entry.get_password(), Err(keyring::Error::NoEntry)));
}
