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

#[test]
fn test_mirror_kv_store() -> Result<(), KvStoreError> {
    let mut ram = RamKvStore::default();

    let ops = &[
        WriteOp::Put("bc".into(), Blob(vec![0, 1, 2, 3])),
        WriteOp::Put("aa".into(), Blob(vec![3, 2, 1, 0])),
        WriteOp::Put("def".into(), Blob(vec![])),
    ];

    ram.update(ops)?;

    let prev_ram_checksum = ram.checksum::<Hasher>()?;

    let mut mirror = RamMirrorKvStore::new(&ram);

    let ops_on_mirror = &[
        WriteOp::Put("bc".into(), Blob(vec![0, 1, 2, 4])),
        WriteOp::Put("dd".into(), Blob(vec![1, 1, 1])),
        WriteOp::Put("ghi".into(), Blob(vec![2, 3])),
    ];

    mirror.update(ops_on_mirror)?;

    let mirror_checksum = mirror.checksum::<Hasher>()?;

    let mirror_ops = mirror.to_ops();

    assert_eq!(ram.checksum::<Hasher>()?, prev_ram_checksum);

    ram.update(&mirror_ops)?;

    assert_eq!(ram.checksum::<Hasher>()?, mirror_checksum);

    Ok(())
}
