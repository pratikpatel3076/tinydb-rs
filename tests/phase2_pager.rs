//! Phase 2 acceptance tests: fixed-size pager over a single file.
//!
//! Written against `tinydb_rs::storage::pager`. Currently fails on `todo!()`.
//!
//! Run just this file with: `cargo test --test phase2_pager`

use tinydb_rs::storage::pager::{Page, Pager, PAGE_SIZE};
use tinydb_rs::{Error, Result};

/// Create a pager in a fresh temp dir. The `TempDir` must outlive the pager, so
/// callers keep it alive in their own scope.
fn create_pager(dir: &tempfile::TempDir, name: &str) -> Result<Pager> {
    Pager::create(dir.path().join(name))
}

/// Build a page whose first bytes are `contents` and whose tail is zero.
fn page_with(contents: &[u8]) -> Page {
    assert!(contents.len() <= PAGE_SIZE);
    let mut page = Page::zeroed();
    page.as_bytes_mut()[..contents.len()].copy_from_slice(contents);
    page
}

#[test]
fn page_size_is_fixed_at_4096() {
    // Compile-time constant: this part is true even before implementation.
    assert_eq!(PAGE_SIZE, 4096);
    // ... but the test must exercise the real API too.
    let mut page = Page::zeroed();
    assert_eq!(page.as_bytes().len(), 4096);
    assert!(page.as_bytes().iter().all(|&b| b == 0));
    page.as_bytes_mut()[0] = 42;
    assert_eq!(page.as_bytes()[0], 42);
}

#[test]
fn allocate_returns_sequential_page_ids_and_grows_the_file() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut pager = create_pager(&dir, "grow.db")?;

    assert_eq!(pager.page_count(), 0);
    assert_eq!(pager.allocate_page()?, 0);
    assert_eq!(pager.allocate_page()?, 1);
    assert_eq!(pager.allocate_page()?, 2);
    assert_eq!(pager.page_count(), 3);

    Ok(())
}

#[test]
fn write_then_read_roundtrips_a_page() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut pager = create_pager(&dir, "roundtrip.db")?;
    let id = pager.allocate_page()?;

    let page = page_with(b"hello, page");
    pager.write_page(id, &page)?;

    let read = pager.read_page(id)?;
    assert_eq!(read.as_bytes(), page.as_bytes());

    Ok(())
}

#[test]
fn pages_are_isolated_from_each_other() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut pager = create_pager(&dir, "isolated.db")?;
    let first = pager.allocate_page()?;
    let second = pager.allocate_page()?;

    pager.write_page(first, &page_with(b"first"))?;
    pager.write_page(second, &page_with(b"second"))?;

    assert_eq!(&pager.read_page(first)?.as_bytes()[..5], b"first");
    assert_eq!(&pager.read_page(second)?.as_bytes()[..6], b"second");

    Ok(())
}

#[test]
fn writing_twice_keeps_the_last_value() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut pager = create_pager(&dir, "overwrite.db")?;
    let id = pager.allocate_page()?;

    pager.write_page(id, &page_with(b"old"))?;
    pager.write_page(id, &page_with(b"new"))?;

    assert_eq!(&pager.read_page(id)?.as_bytes()[..3], b"new");

    Ok(())
}

#[test]
fn reading_or_writing_past_the_end_is_an_error() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut pager = create_pager(&dir, "bounds.db")?;
    let id = pager.allocate_page()?;

    // id + 1 is exactly page_count: allocated space ends at id, so this is out
    // of range, not a lazy extension.
    match pager.read_page(id + 1) {
        Err(Error::PageOutOfRange {
            id: bad,
            page_count: 1,
        }) => assert_eq!(bad, 1),
        other => panic!("expected PageOutOfRange, got {other:?}"),
    }

    match pager.write_page(id + 1, &page_with(b"nope")) {
        Err(Error::PageOutOfRange { .. }) => {}
        other => panic!("expected PageOutOfRange, got {other:?}"),
    }

    Ok(())
}

#[test]
fn flush_persists_pages_across_reopen() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("persist.db");

    {
        let mut pager = Pager::create(&path)?;
        let id = pager.allocate_page()?;
        pager.write_page(id, &page_with(b"durable"))?;
        pager.flush()?;
    }

    let mut reopened = Pager::open(&path)?;
    assert_eq!(reopened.page_count(), 1);
    assert_eq!(&reopened.read_page(0)?.as_bytes()[..7], b"durable");

    Ok(())
}

#[test]
fn create_refuses_to_clobber_an_existing_file() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("exists.db");

    {
        let mut pager = Pager::create(&path)?;
        let id = pager.allocate_page()?;
        pager.write_page(id, &page_with(b"keep me"))?;
        pager.flush()?;
    }

    assert!(
        Pager::create(&path).is_err(),
        "Pager::create must not truncate an existing database"
    );

    // And the original data is still there.
    let mut pager = Pager::open(&path)?;
    assert_eq!(&pager.read_page(0)?.as_bytes()[..7], b"keep me");

    Ok(())
}

#[test]
fn page_path_is_remembered() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("path.db");
    let pager = Pager::create(&path)?;

    assert_eq!(pager.path(), path);

    Ok(())
}
