//! Phase 1 acceptance tests: in-memory key/value store.
//!
//! These tests are written against the *intended* public API of
//! [`tinydb_rs::memtable::MemTable`] and currently fail on `todo!()`. Do not
//! weaken them; make them pass.
//!
//! Run just this file with: `cargo test --test phase1_memtable`

use tinydb_rs::memtable::MemTable;
use tinydb_rs::Result;

#[test]
fn put_then_get_roundtrips() -> Result<()> {
    let mut db = MemTable::new();
    db.put(b"hello", b"world")?;

    assert_eq!(db.get(b"hello")?, Some(b"world".to_vec()));
    assert_eq!(db.get(b"no-such-key")?, None);

    Ok(())
}

#[test]
fn put_overwrites_without_duplicating() -> Result<()> {
    let mut db = MemTable::new();
    db.put(b"k", b"first")?;
    db.put(b"k", b"second")?;

    assert_eq!(db.get(b"k")?, Some(b"second".to_vec()));
    assert_eq!(db.len(), 1);

    Ok(())
}

#[test]
fn delete_reports_whether_the_key_existed() -> Result<()> {
    let mut db = MemTable::new();
    db.put(b"k", b"v")?;

    assert!(db.delete(b"k")?);
    assert_eq!(db.get(b"k")?, None);
    assert_eq!(db.len(), 0);
    // Deleting an absent key is not an error.
    assert!(!db.delete(b"k")?);

    Ok(())
}

#[test]
fn scan_returns_entries_in_sorted_order() -> Result<()> {
    let mut db = MemTable::new();
    // Insert out of order on purpose.
    db.put(b"c", b"3")?;
    db.put(b"a", b"1")?;
    db.put(b"b", b"2")?;

    let rows = db.scan(None, None)?;
    let keys: Vec<&[u8]> = rows.iter().map(|(k, _)| k.as_slice()).collect();
    assert_eq!(
        keys,
        vec![b"a".as_slice(), b"b".as_slice(), b"c".as_slice()]
    );

    Ok(())
}

#[test]
fn scan_bounds_are_start_inclusive_end_exclusive() -> Result<()> {
    let mut db = MemTable::new();
    for key in [b"a", b"b", b"c", b"d"] {
        db.put(key, key)?;
    }

    let rows = db.scan(Some(b"b"), Some(b"d"))?;
    let keys: Vec<&[u8]> = rows.iter().map(|(k, _)| k.as_slice()).collect();
    assert_eq!(keys, vec![b"b".as_slice(), b"c".as_slice()]);

    // Unbounded on either side.
    let tail = db.scan(Some(b"c"), None)?;
    assert_eq!(tail.len(), 2);
    let head = db.scan(None, Some(b"b"))?;
    assert_eq!(head.len(), 1);

    Ok(())
}

#[test]
fn scan_with_start_at_or_after_end_is_empty() -> Result<()> {
    let mut db = MemTable::new();
    db.put(b"b", b"2")?;
    db.put(b"d", b"4")?;

    assert!(db.scan(Some(b"c"), Some(b"c"))?.is_empty());
    assert!(db.scan(Some(b"d"), Some(b"b"))?.is_empty());

    Ok(())
}

#[test]
fn byte_order_is_unsigned() -> Result<()> {
    let mut db = MemTable::new();
    // 0x7F < 0x80 < 0xFF when bytes are compared as u8; a signed comparison
    // would order 0x80 and 0xFF before 0x7F.
    db.put([0xFF], b"last")?;
    db.put([0x7F], b"first")?;
    db.put([0x80], b"middle")?;

    let rows = db.scan(None, None)?;
    let keys: Vec<u8> = rows.iter().map(|(k, _)| k[0]).collect();
    assert_eq!(keys, vec![0x7F, 0x80, 0xFF]);

    Ok(())
}

#[test]
fn empty_values_are_distinct_from_missing_keys() -> Result<()> {
    let mut db = MemTable::new();
    db.put(b"empty", b"")?;

    assert_eq!(db.get(b"empty")?, Some(Vec::new()));
    assert_eq!(db.get(b"absent")?, None);

    Ok(())
}

#[test]
fn len_tracks_live_entries() -> Result<()> {
    let mut db = MemTable::new();
    assert!(db.is_empty());

    db.put(b"a", b"1")?;
    db.put(b"b", b"2")?;
    assert_eq!(db.len(), 2);
    assert!(!db.is_empty());

    db.delete(b"a")?;
    assert_eq!(db.len(), 1);

    Ok(())
}
