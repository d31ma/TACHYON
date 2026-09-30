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
fn different_content_and_encodings_keep_a_bounded_cache() {
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
    assert!(fs::read_dir(root.path()).expect("entries").count() <= 64);
}
