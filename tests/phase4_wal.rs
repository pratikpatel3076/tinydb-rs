//! Phase 4 acceptance tests: write-ahead log and crash recovery.
//!
//! The recovery test at the bottom is the important one: it simulates a crash by
//! never writing the page to the database file, then checks that replaying the
//! WAL repairs it.
//!
//! Currently fails on `todo!()`. Run with:
//! `cargo test --test phase4_wal`

use std::fs::OpenOptions;
use std::io::Write;

use tinydb_rs::storage::pager::{Page, Pager, PAGE_SIZE};
use tinydb_rs::wal::{recover, Wal, WalRecord};
use tinydb_rs::Result;

/// A page whose first bytes are `contents`.
fn page_image(contents: &[u8]) -> Page {
    assert!(contents.len() <= PAGE_SIZE);
    let mut page = Page::zeroed();
    page.as_bytes_mut()[..contents.len()].copy_from_slice(contents);
    page
}

#[test]
fn append_assigns_strictly_increasing_lsns() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut wal = Wal::create(dir.path().join("monotonic.wal"))?;

    let first = wal.append(&WalRecord::Begin { txn: 1 })?;
    let second = wal.append(&WalRecord::Commit { txn: 1 })?;
    let third = wal.append(&WalRecord::Begin { txn: 2 })?;

    assert!(first < second, "lsns must increase: {first} then {second}");
    assert!(second < third, "lsns must increase: {second} then {third}");

    Ok(())
}

#[test]
fn records_roundtrip_in_order() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("roundtrip.wal");

    let written = vec![
        WalRecord::Begin { txn: 3 },
        WalRecord::PageWrite {
            txn: 3,
            page_id: 0,
            page: page_image(b"page zero").as_bytes().to_vec(),
        },
        WalRecord::Commit { txn: 3 },
    ];

    {
        let mut wal = Wal::create(&path)?;
        for record in &written {
            wal.append(record)?;
        }
        wal.sync()?;
    }

    // A new handle must see the same stream, in the same order.
    let mut wal = Wal::open(&path)?;
    let read: Vec<WalRecord> = wal.records()?.into_iter().map(|(_, r)| r).collect();
    assert_eq!(read, written);

    Ok(())
}

#[test]
fn record_accessors_expose_txn_and_page() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut wal = Wal::create(dir.path().join("accessors.wal"))?;

    let write = WalRecord::PageWrite {
        txn: 9,
        page_id: 12,
        page: page_image(b"x").as_bytes().to_vec(),
    };
    wal.append(&write)?;

    assert_eq!(write.txn(), 9);
    assert_eq!(write.page_id(), Some(12));
    assert_eq!(WalRecord::Commit { txn: 9 }.page_id(), None);

    Ok(())
}

/// The crash-recovery test.
///
/// Timeline:
/// 1. page 0 is written to disk with "before" and fsynced;
/// 2. a transaction logs a full after-image "after" and commits (WAL fsynced);
/// 3. the process dies *before* the page reaches the database file;
/// 4. recovery must replay the committed image.
#[test]
fn crash_recovery_replays_committed_page_writes() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let db_path = dir.path().join("crash.db");
    let wal_path = dir.path().join("crash.wal");

    {
        let mut pager = Pager::create(&db_path)?;
        let mut wal = Wal::create(&wal_path)?;

        let page_id = pager.allocate_page()?;
        pager.write_page(page_id, &page_image(b"before"))?;
        pager.flush()?;

        let after = page_image(b"after").as_bytes().to_vec();
        wal.append(&WalRecord::Begin { txn: 7 })?;
        wal.append(&WalRecord::PageWrite {
            txn: 7,
            page_id,
            page: after,
        })?;
        wal.append(&WalRecord::Commit { txn: 7 })?;
        wal.sync()?;

        // Deliberately no pager.write_page(page_id, ...): simulate a crash
        // between "WAL says committed" and "dirty page reached the file".
    }

    let report = recover(&db_path, &wal_path)?;
    assert_eq!(
        report.pages_replayed, 1,
        "committed page write must be replayed"
    );
    assert_eq!(report.transactions_committed, 1);

    let mut pager = Pager::open(&db_path)?;
    assert_eq!(&pager.read_page(0)?.as_bytes()[..5], b"after");

    Ok(())
}

#[test]
fn recovery_discards_uncommitted_transactions() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let db_path = dir.path().join("uncommitted.db");
    let wal_path = dir.path().join("uncommitted.wal");

    {
        let mut pager = Pager::create(&db_path)?;
        let mut wal = Wal::create(&wal_path)?;

        let page_id = pager.allocate_page()?;
        pager.write_page(page_id, &page_image(b"committed"))?;
        pager.flush()?;

        // Begin + page write, but no Commit.
        wal.append(&WalRecord::Begin { txn: 8 })?;
        wal.append(&WalRecord::PageWrite {
            txn: 8,
            page_id,
            page: page_image(b"uncommitted").as_bytes().to_vec(),
        })?;
        wal.sync()?;
    }

    let report = recover(&db_path, &wal_path)?;
    assert_eq!(report.pages_replayed, 0);
    assert_eq!(report.transactions_discarded, 1);

    let mut pager = Pager::open(&db_path)?;
    assert_eq!(&pager.read_page(0)?.as_bytes()[..9], b"committed");

    Ok(())
}

#[test]
fn torn_tail_is_tolerated_not_fatal() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let db_path = dir.path().join("torn.db");
    let wal_path = dir.path().join("torn.wal");

    {
        let mut pager = Pager::create(&db_path)?;
        let mut wal = Wal::create(&wal_path)?;

        let page_id = pager.allocate_page()?;
        pager.write_page(page_id, &page_image(b"before"))?;
        pager.flush()?;

        wal.append(&WalRecord::Begin { txn: 1 })?;
        wal.append(&WalRecord::PageWrite {
            txn: 1,
            page_id,
            page: page_image(b"after").as_bytes().to_vec(),
        })?;
        wal.append(&WalRecord::Commit { txn: 1 })?;
        wal.sync()?;
    }

    // Simulate a torn write: garbage at the end of the log.
    OpenOptions::new()
        .append(true)
        .open(&wal_path)?
        .write_all(b"\x00\x00\x00\x20incomplete-record")?;

    let report = recover(&db_path, &wal_path)?;
    assert!(
        report.torn_tail,
        "partial record at the tail must be reported"
    );
    assert_eq!(report.pages_replayed, 1, "valid prefix must still replay");

    let mut pager = Pager::open(&db_path)?;
    assert_eq!(&pager.read_page(0)?.as_bytes()[..5], b"after");

    Ok(())
}

#[test]
fn recovery_is_idempotent() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let db_path = dir.path().join("idempotent.db");
    let wal_path = dir.path().join("idempotent.wal");

    {
        let mut pager = Pager::create(&db_path)?;
        let mut wal = Wal::create(&wal_path)?;
        let page_id = pager.allocate_page()?;
        wal.append(&WalRecord::Begin { txn: 1 })?;
        wal.append(&WalRecord::PageWrite {
            txn: 1,
            page_id,
            page: page_image(b"once").as_bytes().to_vec(),
        })?;
        wal.append(&WalRecord::Commit { txn: 1 })?;
        wal.sync()?;
    }

    let first = recover(&db_path, &wal_path)?;
    assert_eq!(first.pages_replayed, 1);

    // The WAL was truncated after a successful checkpoint, so this is a no-op.
    let second = recover(&db_path, &wal_path)?;
    assert_eq!(second.pages_replayed, 0);

    let mut pager = Pager::open(&db_path)?;
    assert_eq!(&pager.read_page(0)?.as_bytes()[..4], b"once");

    Ok(())
}

#[test]
fn missing_wal_is_a_noop() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let db_path = dir.path().join("clean.db");
    let wal_path = dir.path().join("does-not-exist.wal");

    {
        let mut pager = Pager::create(&db_path)?;
        let page_id = pager.allocate_page()?;
        pager.write_page(page_id, &page_image(b"clean shutdown"))?;
        pager.flush()?;
    }

    let report = recover(&db_path, &wal_path)?;
    assert_eq!(report.pages_replayed, 0);
    assert!(!report.torn_tail);

    let mut pager = Pager::open(&db_path)?;
    assert_eq!(&pager.read_page(0)?.as_bytes()[..15], b"clean shutdown");

    Ok(())
}
