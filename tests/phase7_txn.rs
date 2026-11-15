//! Phase 7 acceptance tests: transactions.
//!
//! Starts with the single-writer contract, then checks snapshot isolation for the
//! MVCC stretch goal. Currently fails on `todo!()`. Run with:
//! `cargo test --test phase7_txn`

use tinydb_rs::txn::{IsolationLevel, TransactionManager, TxnState};
use tinydb_rs::{Error, Result};

#[test]
fn commit_makes_writes_visible() -> Result<()> {
    let mut manager = TransactionManager::new(IsolationLevel::SingleWriter);

    let mut writer = manager.begin()?;
    assert_eq!(writer.state(), TxnState::Active);
    manager.put(&mut writer, b"k", b"v")?;
    // Reads inside the transaction see its own uncommitted write.
    assert_eq!(manager.get(&writer, b"k")?, Some(b"v".to_vec()));
    manager.commit(writer)?;

    let reader = manager.begin()?;
    assert_eq!(manager.get(&reader, b"k")?, Some(b"v".to_vec()));
    manager.abort(reader)?;

    Ok(())
}

#[test]
fn abort_discards_all_writes() -> Result<()> {
    let mut manager = TransactionManager::new(IsolationLevel::SingleWriter);

    let mut writer = manager.begin()?;
    manager.put(&mut writer, b"keep-out", b"value")?;
    manager.abort(writer)?;

    let reader = manager.begin()?;
    assert_eq!(manager.get(&reader, b"keep-out")?, None);
    manager.abort(reader)?;

    Ok(())
}

#[test]
fn delete_is_only_effective_after_commit() -> Result<()> {
    let mut manager = TransactionManager::new(IsolationLevel::SingleWriter);

    let mut seed = manager.begin()?;
    manager.put(&mut seed, b"k", b"v")?;
    manager.commit(seed)?;

    let mut writer = manager.begin()?;
    assert!(manager.delete(&mut writer, b"k")?);
    assert!(!manager.delete(&mut writer, b"missing")?);
    assert_eq!(manager.get(&writer, b"k")?, None);
    manager.commit(writer)?;

    let reader = manager.begin()?;
    assert_eq!(manager.get(&reader, b"k")?, None);
    manager.abort(reader)?;

    Ok(())
}

#[test]
fn single_writer_rejects_a_second_concurrent_transaction() -> Result<()> {
    let mut manager = TransactionManager::new(IsolationLevel::SingleWriter);

    let first = manager.begin()?;
    let first_id = first.id();
    assert_eq!(manager.active_txn(), Some(first_id));

    match manager.begin() {
        Err(Error::TxnConflict { txn, .. }) => assert_eq!(txn, first_id),
        other => panic!("expected TxnConflict, got {other:?}"),
    }
    // The rejected begin must not leak or replace the active transaction.
    assert_eq!(manager.active_txn(), Some(first_id));

    manager.abort(first)?;
    assert_eq!(manager.active_txn(), None);
    assert_eq!(manager.isolation(), IsolationLevel::SingleWriter);

    Ok(())
}

#[test]
fn empty_commit_and_abort_are_legal() -> Result<()> {
    let mut manager = TransactionManager::new(IsolationLevel::SingleWriter);

    let empty = manager.begin()?;
    manager.commit(empty)?;
    let empty = manager.begin()?;
    manager.abort(empty)?;

    Ok(())
}

#[test]
fn snapshot_isolation_does_not_see_later_commits() -> Result<()> {
    let mut manager = TransactionManager::new(IsolationLevel::SnapshotIsolation);

    // The reader's snapshot is taken now, before the writer commits.
    let reader = manager.begin()?;

    let mut writer = manager.begin()?;
    manager.put(&mut writer, b"k", b"v")?;
    manager.commit(writer)?;

    assert_eq!(manager.get(&reader, b"k")?, None);
    manager.abort(reader)?;

    // A transaction begun after the commit must see the value.
    let fresh = manager.begin()?;
    assert_eq!(manager.get(&fresh, b"k")?, Some(b"v".to_vec()));
    manager.abort(fresh)?;

    Ok(())
}

#[test]
fn last_write_within_a_transaction_wins() -> Result<()> {
    let mut manager = TransactionManager::new(IsolationLevel::SingleWriter);

    let mut writer = manager.begin()?;
    manager.put(&mut writer, b"k", b"first")?;
    manager.put(&mut writer, b"k", b"second")?;
    manager.commit(writer)?;

    let reader = manager.begin()?;
    assert_eq!(manager.get(&reader, b"k")?, Some(b"second".to_vec()));
    manager.abort(reader)?;

    Ok(())
}
