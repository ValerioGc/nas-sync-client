use super::*;

use crate::test_support::Scratch;

#[test]
fn a_missing_file_is_nothing_and_a_written_one_reads_back() {
    let scratch = Scratch::new("storage");
    let path = scratch.0.join("nested/value.json");
    assert_eq!(read::<Vec<u32>>(&path), Ok(None));
    write(&path, &vec![1, 2, 3]).unwrap();
    assert_eq!(read::<Vec<u32>>(&path), Ok(Some(vec![1, 2, 3])));
    write(&path, &vec![4]).unwrap();
    assert_eq!(read::<Vec<u32>>(&path), Ok(Some(vec![4])));
}

#[test]
fn a_corrupt_or_oversized_file_is_refused() {
    let scratch = Scratch::new("storage");
    let path = scratch.0.join("value.json");
    fs::write(&path, "not json").unwrap();
    assert_eq!(
        read::<Vec<u32>>(&path),
        Err(ClientError::StorageUnavailable)
    );
    fs::write(&path, vec![b' '; (MAX_BYTES + 1) as usize]).unwrap();
    assert_eq!(
        read::<Vec<u32>>(&path),
        Err(ClientError::StorageUnavailable)
    );
    fs::create_dir_all(scratch.0.join("folder.json")).unwrap();
    assert_eq!(
        read::<Vec<u32>>(&scratch.0.join("folder.json")),
        Err(ClientError::StorageUnavailable)
    );
}
