use super::{CompressionCache, ContentEncoding};
use std::fs;

#[test]
fn another_cache_owner_reuses_valid_encoded_content_without_previous_output() {
    let root = tempfile::tempdir().expect("cache root");
    let bytes = b"const message = 'unchanged runtime';\n".repeat(128);
    let cache = CompressionCache::at(root.path().to_path_buf());
    cache.encode(&bytes, ContentEncoding::GZIP).expect("encode");
    let entries: Vec<_> = fs::read_dir(root.path()).expect("entries").collect();
    assert_eq!(entries.len(), 1, "fresh builds persist compressed content");
    let entry = entries[0].as_ref().expect("entry").path();
    let mut reused = fs::read(&entry).expect("persisted header and gzip");
    reused[32 + 4] = 1;
    fs::write(entry, &reused).expect("valid alternate gzip header");
    let other = CompressionCache::at(root.path().to_path_buf());
    assert_eq!(
        other.encode(&bytes, ContentEncoding::GZIP).expect("reuse"),
        reused[32..]
    );
}

#[test]
fn corrupt_oversized_and_wrong_content_entries_are_replaced() {
    let root = tempfile::tempdir().expect("cache root");
    let bytes = b"const message = 'runtime';\n".repeat(128);
    let cache = CompressionCache::at(root.path().to_path_buf());
    let expected = cache.encode(&bytes, ContentEncoding::GZIP).expect("encode");
    let entry = fs::read_dir(root.path())
        .expect("entries")
        .next()
        .expect("one entry")
        .expect("entry")
        .path();
    for content in [b"broken".to_vec(), vec![0; 4 * 1024 * 1024 + 33]] {
        fs::write(&entry, content).expect("invalid cached artifact");
        assert_eq!(
            cache
                .encode(&bytes, ContentEncoding::GZIP)
                .expect("recover"),
            expected
        );
    }
    let mut changed = fs::read(&entry).expect("cache entry");
    changed[32..].fill(0);
    fs::write(&entry, &changed).expect("corrupt compressed content");
    assert_eq!(
        cache
            .encode(&bytes, ContentEncoding::GZIP)
            .expect("recover payload"),
        expected
    );
    let mut changed = fs::read(&entry).expect("repaired cache entry");
    changed[0] ^= 1;
    fs::write(&entry, changed).expect("different content key");
    assert_eq!(
        cache
            .encode(&bytes, ContentEncoding::GZIP)
            .expect("recover key"),
        expected
    );
}

#[test]
fn different_content_and_encodings_each_keep_their_own_entry() {
    let root = tempfile::tempdir().expect("cache root");
    let cache = CompressionCache::at(root.path().to_path_buf());
    for value in 0..100 {
        let bytes = format!("const message = '{value}';\n")
            .repeat(128)
            .into_bytes();
        let gzip = cache.encode(&bytes, ContentEncoding::GZIP).expect("gzip");
        assert!(ContentEncoding::GZIP.matches(&gzip, &bytes));
    }
    let bytes = b"different Brotli runtime\n".repeat(128);
    let brotli = cache
        .encode(&bytes, ContentEncoding::BROTLI)
        .expect("brotli");
    assert!(ContentEncoding::BROTLI.matches(&brotli, &bytes));
    assert_eq!(fs::read_dir(root.path()).expect("entries").count(), 101);
}

#[test]
fn trimming_removes_the_oldest_entries_until_the_cache_fits() {
    let root = tempfile::tempdir().expect("cache root");
    let cache = CompressionCache::at(root.path().to_path_buf());
    let mut written: Vec<std::path::PathBuf> = Vec::new();
    for age in 0..10_u64 {
        let bytes = format!("const message = '{age}';\n")
            .repeat(128)
            .into_bytes();
        cache.encode(&bytes, ContentEncoding::GZIP).expect("gzip");
        let entry = fs::read_dir(root.path())
            .expect("entries")
            .map(|entry| entry.expect("entry").path())
            .find(|path| !written.contains(path))
            .expect("new entry");
        let stamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000 + age);
        fs::File::options()
            .write(true)
            .open(&entry)
            .and_then(|file| file.set_modified(stamp))
            .expect("entry age");
        written.push(entry);
    }
    let total: u64 = written
        .iter()
        .map(|path| fs::metadata(path).expect("size").len())
        .sum();
    cache.trim_to(total / 2);
    let kept: u64 = written
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .sum();
    assert!(kept <= total / 2, "{kept} > {}", total / 2);
    assert!(!written[0].exists(), "the oldest entry goes first");
    assert!(written[9].exists(), "the newest entry stays");
}

#[test]
fn a_cache_that_cannot_be_written_still_returns_the_encoded_content() {
    let parent = tempfile::tempdir().expect("cache parent");
    let blocked = parent.path().join("not-a-directory");
    fs::write(&blocked, "file").expect("file where the cache would go");
    let cache = CompressionCache::at(blocked.join("cache"));
    let bytes = b"const message = 'read-only cache';\n".repeat(128);
    let encoded = cache
        .encode(&bytes, ContentEncoding::GZIP)
        .expect("compression succeeds without a cache");
    assert!(ContentEncoding::GZIP.matches(&encoded, &bytes));
}
