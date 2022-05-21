use super::*;
use tempdir::TempDir;

fn temp_disk_store() -> Result<LevelDbKvStore, KvStoreError> {
    LevelDbKvStore::new(TempDir::new("bazuka_test").unwrap().path(), 64)
}

#[test]
fn test_ram_and_disk_db_consistency() -> Result<(), KvStoreError> {
    let mut ram = RamKvStore::default();
    let mut disk = temp_disk_store()?;

    assert_eq!(ram.checksum::<Hasher>()?, disk.checksum::<Hasher>()?);

    let ops = &[
        WriteOp::Put("bc".into(), Blob(vec![0, 1, 2, 3])),
        WriteOp::Put("aa".into(), Blob(vec![3, 2, 1, 0])),
        WriteOp::Put("def".into(), Blob(vec![])),
    ];

    ram.update(ops)?;
    disk.update(ops)?;

    assert_eq!(ram.checksum::<Hasher>()?, disk.checksum::<Hasher>()?);

    let new_ops = &[
        WriteOp::Remove("aa".into()),
        WriteOp::Put("def".into(), Blob(vec![1, 1, 1, 2])),
        WriteOp::Put("ghi".into(), Blob(vec![3, 3, 3, 3])),
    ];

    ram.update(new_ops)?;
    disk.update(new_ops)?;

    assert_eq!(ram.checksum::<Hasher>()?, disk.checksum::<Hasher>()?);

    Ok(())
}
