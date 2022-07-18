use super::*;

#[cfg(feature = "db")]
use tempdir::TempDir;

#[cfg(feature = "db")]
fn temp_disk_store() -> Result<LevelDbKvStore, KvStoreError> {
    LevelDbKvStore::new(TempDir::new("bazuka_test").unwrap().path(), 64)
}

#[test]
#[cfg(feature = "db")]
fn test_ram_and_disk_pair_prefix() -> Result<(), KvStoreError> {
    let mut ram = RamKvStore::default();
    let mut disk = temp_disk_store()?;

    assert_eq!(ram.checksum::<Hasher>()?, disk.checksum::<Hasher>()?);

    let ops = &[
        WriteOp::Put("bc".into(), Blob(vec![0, 1, 2, 3])),
        WriteOp::Put("aa".into(), Blob(vec![3, 2, 1, 0])),
        WriteOp::Put("a0a".into(), Blob(vec![])),
        WriteOp::Put("bge".into(), Blob(vec![])),
        WriteOp::Put("def".into(), Blob(vec![])),
    ];

    ram.update(ops)?;
    disk.update(ops)?;

    assert_eq!(disk.pairs("".into())?.len(), 5);
    assert_eq!(ram.pairs("".into())?.len(), 5);
    assert_eq!(disk.pairs("a".into())?.len(), 2);
    assert_eq!(ram.pairs("a".into())?.len(), 2);
    assert_eq!(disk.pairs("b".into())?.len(), 2);
    assert_eq!(ram.pairs("b".into())?.len(), 2);
    assert_eq!(disk.pairs("d".into())?.len(), 1);
    assert_eq!(ram.pairs("d".into())?.len(), 1);
    assert_eq!(disk.pairs("a0".into())?.len(), 1);
    assert_eq!(ram.pairs("a0".into())?.len(), 1);
    assert_eq!(disk.pairs("a1".into())?.len(), 0);
    assert_eq!(ram.pairs("a1".into())?.len(), 0);

    Ok(())
}

#[test]
#[cfg(feature = "db")]
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

#[test]
fn test_mirror_rollback() -> Result<(), KvStoreError> {
    let mut ram = RamKvStore::default();

    let ops = &[
        WriteOp::Put("bc".into(), Blob(vec![0, 1, 2, 3])),
        WriteOp::Put("aa".into(), Blob(vec![3, 2, 1, 0])),
        WriteOp::Put("def".into(), Blob(vec![])),
    ];

    ram.update(ops)?;

    let mirror1 = ram.mirror();
    assert_eq!(mirror1.rollback()?, vec![]);

    let mut mirror2 = ram.mirror();
    mirror2.update(&[WriteOp::Remove("kk".into())])?;
    assert_eq!(mirror2.rollback()?, vec![WriteOp::Remove("kk".into())]);

    let mut mirror3 = ram.mirror();
    mirror3.update(&[
        WriteOp::Put("bc".into(), Blob(vec![3, 2, 1])),
        WriteOp::Put("gg".into(), Blob(vec![2, 2, 2, 2])),
        WriteOp::Put("fre".into(), Blob(vec![1, 1])),
        WriteOp::Remove("aa".into()),
    ])?;
    let mut mirror3_rollback = mirror3.rollback()?;
    mirror3_rollback.sort_by_key(|v| match v {
        WriteOp::Put(k, _) => k.clone(),
        WriteOp::Remove(k) => k.clone(),
    });
    assert_eq!(
        mirror3_rollback,
        vec![
            WriteOp::Put("aa".into(), Blob(vec![3, 2, 1, 0])),
            WriteOp::Put("bc".into(), Blob(vec![0, 1, 2, 3])),
            WriteOp::Remove("fre".into()),
            WriteOp::Remove("gg".into()),
        ]
    );

    Ok(())
}
