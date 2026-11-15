//! Phase 5 acceptance tests: buffer pool and eviction.
//!
//! Currently fails on `todo!()`. Run with:
//! `cargo test --test phase5_buffer_pool`

use tempfile::TempDir;
use tinydb_rs::storage::buffer_pool::{BufferPool, DEFAULT_CAPACITY};
use tinydb_rs::storage::pager::{Pager, PAGE_SIZE};
use tinydb_rs::{Error, Result};

fn create_pool(dir: &TempDir, name: &str, capacity: usize) -> Result<BufferPool> {
    let pager = Pager::create(dir.path().join(name))?;
    Ok(BufferPool::new(pager, capacity))
}

#[test]
fn allocate_then_fetch_is_a_hit() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut pool = create_pool(&dir, "hit.db", 4)?;

    let id = pool.allocate()?;
    {
        let page = pool.fetch(id)?;
        assert_eq!(page.as_bytes().len(), PAGE_SIZE);
    }
    pool.unpin(id);
    pool.unpin(id);

    let stats = pool.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 0);
    assert_eq!(pool.capacity(), 4);

    Ok(())
}

#[test]
fn fetch_after_reopen_is_a_miss() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("miss.db");

    {
        let mut pool = create_pool(&dir, "miss.db", 4)?;
        let id = pool.allocate()?;
        pool.unpin(id);
        pool.flush_all()?;
    }

    let pager = Pager::open(&path)?;
    let mut pool = BufferPool::new(pager, 4);
    {
        let page = pool.fetch(0)?;
        assert_eq!(page.as_bytes().len(), PAGE_SIZE);
    }

    assert_eq!(pool.stats().misses, 1);
    assert_eq!(pool.stats().hits, 0);

    Ok(())
}

#[test]
fn lru_evicts_the_least_recently_used_unpinned_page() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("lru.db");
    let mut pool = create_pool(&dir, "lru.db", 2)?;

    let first = pool.allocate()?;
    pool.unpin(first);
    let second = pool.allocate()?;
    pool.unpin(second);

    // Mark `second` dirty so eviction must write it back.
    {
        let page = pool.fetch(second)?;
        page.as_bytes_mut()[..3].copy_from_slice(b"two");
    }
    pool.unpin(second);

    // Touch `first` so that `second` becomes the least recently used page.
    {
        let _page = pool.fetch(first)?;
    }
    pool.unpin(first);

    // Capacity is 2, both frames pinned=0: this must evict `second`.
    let third = pool.allocate()?;
    pool.unpin(third);

    assert_eq!(pool.stats().evictions, 1);
    pool.flush_all()?;
    drop(pool);

    // The evicted-but-dirty page must have been written back.
    let mut pager = Pager::open(&path)?;
    assert_eq!(&pager.read_page(second)?.as_bytes()[..3], b"two");
    assert_eq!(pager.page_count(), 3);

    Ok(())
}

#[test]
fn pinned_pages_are_never_evicted() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let mut pool = create_pool(&dir, "pinned.db", 1)?;

    let pinned = pool.allocate()?;

    // The only frame is pinned; growing the pool must fail rather than corrupt.
    match pool.allocate() {
        Err(Error::PoolExhausted { capacity: 1 }) => {}
        other => panic!("expected PoolExhausted, got {other:?}"),
    }
    assert_eq!(pool.stats().evictions, 0);
    assert_eq!(pool.stats().writes, 0);

    // After unpinning, the frame is reusable.
    pool.unpin(pinned);
    let next = pool.allocate()?;
    assert_ne!(pinned, next);

    Ok(())
}

#[test]
fn flush_all_persists_every_dirty_frame() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("flush.db");

    {
        let mut pool = create_pool(&dir, "flush.db", 4)?;
        for i in 0..3u8 {
            let id = pool.allocate()?;
            {
                let page = pool.fetch(id)?; // second pin (allocate pinned once)
                page.as_bytes_mut()[0] = i + 1;
            }
            pool.unpin(id);
            pool.unpin(id);
        }
        assert_eq!(pool.page_count(), 3);
        pool.flush_all()?;
    }

    let mut pager = Pager::open(&path)?;
    assert_eq!(pager.page_count(), 3);
    for i in 0..3u64 {
        assert_eq!(pager.read_page(i)?.as_bytes()[0], i as u8 + 1);
    }

    Ok(())
}

#[test]
fn capacity_and_stats_start_sane() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let pool = create_pool(&dir, "fresh.db", DEFAULT_CAPACITY)?;

    assert_eq!(pool.capacity(), DEFAULT_CAPACITY);
    assert_eq!(pool.page_count(), 0);
    assert_eq!(pool.stats().hits, 0);
    assert_eq!(pool.stats().misses, 0);
    assert_eq!(pool.stats().evictions, 0);
    assert_eq!(pool.stats().writes, 0);

    Ok(())
}
