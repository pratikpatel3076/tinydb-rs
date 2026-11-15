//! Phase 3 acceptance tests: B+tree over pages.
//!
//! The randomized test at the bottom is the real specification: the tree must
//! agree with `std::collections::BTreeMap` for every operation sequence.
//! Currently fails on `todo!()`.
//!
//! Run just this file with: `cargo test --test phase3_btree`

use std::collections::BTreeMap;

use proptest::prelude::*;
use tempfile::TempDir;
use tinydb_rs::index::btree::BTree;
use tinydb_rs::storage::buffer_pool::BufferPool;
use tinydb_rs::storage::pager::Pager;
use tinydb_rs::Result;

fn key(n: u16) -> [u8; 2] {
    n.to_be_bytes()
}

/// Fresh on-disk tree. 8 frames is small enough that eviction happens, which is
/// what we want to exercise alongside splits.
fn create_tree(dir: &TempDir) -> Result<BTree> {
    let pager = Pager::create(dir.path().join("btree.db"))?;
    BTree::create(BufferPool::new(pager, 8))
}

#[test]
fn insert_then_get_roundtrips() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut tree = create_tree(&dir)?;

    tree.insert(key(7), b"seven")?;

    assert_eq!(tree.get(&key(7))?, Some(b"seven".to_vec()));
    assert_eq!(tree.get(&key(8))?, None);

    Ok(())
}

#[test]
fn insert_overwrites_an_existing_key() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut tree = create_tree(&dir)?;

    tree.insert(key(1), b"old")?;
    tree.insert(key(1), b"new")?;

    assert_eq!(tree.get(&key(1))?, Some(b"new".to_vec()));
    assert_eq!(tree.key_count()?, 1);

    Ok(())
}

#[test]
fn scan_returns_keys_in_ascending_order() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut tree = create_tree(&dir)?;

    for n in [9u16, 1, 5, 3, 7] {
        tree.insert(key(n), n.to_be_bytes())?;
    }

    let scanned: Vec<Vec<u8>> = tree.scan(None, None)?.into_iter().map(|(k, _)| k).collect();
    let expected: Vec<Vec<u8>> = [1u16, 3, 5, 7, 9]
        .iter()
        .map(|n| key(*n).to_vec())
        .collect();
    assert_eq!(scanned, expected);

    Ok(())
}

#[test]
fn scan_bounds_are_half_open() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut tree = create_tree(&dir)?;

    for n in 0u16..10 {
        tree.insert(key(n), n.to_be_bytes())?;
    }

    let rows = tree.scan(Some(&key(3)), Some(&key(6)))?;
    let keys: Vec<u16> = rows
        .iter()
        .map(|(k, _)| u16::from_be_bytes([k[0], k[1]]))
        .collect();
    assert_eq!(keys, vec![3, 4, 5]);

    // Empty ranges are valid.
    assert!(tree.scan(Some(&key(5)), Some(&key(5)))?.is_empty());
    assert!(tree.scan(Some(&key(8)), Some(&key(2)))?.is_empty());

    Ok(())
}

#[test]
fn many_sequential_inserts_split_leaves_and_stay_searchable() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut tree = create_tree(&dir)?;

    // Ascending order is the worst case for a B+tree split path: every insert
    // goes to the rightmost leaf, so every split propagates to the parent.
    for n in 0u16..1_000 {
        tree.insert(key(n), n.to_be_bytes())?;
    }

    assert_eq!(tree.key_count()?, 1_000);
    for n in 0u16..1_000 {
        assert_eq!(
            tree.get(&key(n))?.map(|v| u16::from_be_bytes([v[0], v[1]])),
            Some(n),
            "lost key {n} after splits"
        );
    }

    // The full scan must still be ordered and complete.
    let all = tree.scan(None, None)?;
    assert_eq!(all.len(), 1_000);
    for (index, (k, _)) in all.iter().enumerate() {
        assert_eq!(k.as_slice(), key(index as u16));
    }

    Ok(())
}

#[test]
fn large_values_force_splits_and_still_roundtrip() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut tree = create_tree(&dir)?;

    // ~910-byte entries: roughly four per 4 KiB page, so 40 keys means many
    // splits without any single value being too large.
    let value = vec![0xABu8; 910];
    for n in 0u16..40 {
        tree.insert(key(n), value.clone())?;
    }

    for n in 0u16..40 {
        assert_eq!(tree.get(&key(n))?, Some(value.clone()));
    }

    Ok(())
}

#[test]
fn delete_removes_only_the_requested_key() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut tree = create_tree(&dir)?;

    for n in 0u16..20 {
        tree.insert(key(n), n.to_be_bytes())?;
    }

    assert!(tree.delete(&key(7))?);
    assert!(!tree.delete(&key(7))?);
    assert_eq!(tree.get(&key(7))?, None);

    // Neighbours and scan integrity survive.
    assert_eq!(tree.get(&key(6))?, Some(6u16.to_be_bytes().to_vec()));
    assert_eq!(tree.get(&key(8))?, Some(8u16.to_be_bytes().to_vec()));
    let keys: Vec<u16> = tree
        .scan(None, None)?
        .iter()
        .map(|(k, _)| u16::from_be_bytes([k[0], k[1]]))
        .collect();
    assert_eq!(keys.len(), 19);
    assert!(!keys.contains(&7));

    Ok(())
}

#[test]
fn flush_then_reopen_preserves_the_tree() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("btree.db");

    {
        let pager = Pager::create(&path)?;
        let mut tree = BTree::create(BufferPool::new(pager, 8))?;
        for n in 0u16..100 {
            tree.insert(key(n), format!("value-{n}"))?;
        }
        tree.flush()?;
    }

    // Simulate restart: new pager, new pool, same file.
    let pager = Pager::open(&path)?;
    let mut tree = BTree::open(BufferPool::new(pager, 8))?;

    assert_eq!(tree.key_count()?, 100);
    assert_eq!(tree.get(&key(42))?, Some(b"value-42".to_vec()));
    assert_eq!(tree.scan(None, None)?.len(), 100);

    Ok(())
}

#[test]
fn empty_tree_is_a_valid_tree() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut tree = create_tree(&dir)?;

    assert_eq!(tree.key_count()?, 0);
    assert!(tree.scan(None, None)?.is_empty());
    assert_eq!(tree.get(b"anything")?, None);
    assert!(!tree.delete(b"anything")?);

    Ok(())
}

// ---------------------------------------------------------------------------
// Property test: the tree is a BTreeMap. Nothing more, nothing less.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Op {
    Put(u16, u16),
    Delete(u16),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        (any::<u16>(), any::<u16>()).prop_map(|(k, v)| Op::Put(k, v)),
        any::<u16>().prop_map(Op::Delete),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn btree_matches_std_btreemap(ops in prop::collection::vec(op_strategy(), 0..150)) {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut tree = create_tree(&dir).expect("create tree");
        let mut model: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();

        for op in ops {
            match op {
                Op::Put(k, v) => {
                    let key_bytes = key(k).to_vec();
                    let value_bytes = v.to_be_bytes().to_vec();
                    tree.insert(key_bytes.clone(), value_bytes.clone())
                        .expect("insert");
                    model.insert(key_bytes, value_bytes);
                }
                Op::Delete(k) => {
                    let key_bytes = key(k).to_vec();
                    let existed = model.remove(&key_bytes).is_some();
                    prop_assert_eq!(
                        tree.delete(&key_bytes).expect("delete"),
                        existed,
                        "delete({:?}) disagrees with the model", k
                    );
                }
            }
        }

        prop_assert_eq!(
            tree.key_count().expect("key_count"),
            model.len() as u64
        );

        let scanned = tree.scan(None, None).expect("scan");
        let expected: Vec<(Vec<u8>, Vec<u8>)> = model.into_iter().collect();
        prop_assert_eq!(scanned, expected);
    }
}
