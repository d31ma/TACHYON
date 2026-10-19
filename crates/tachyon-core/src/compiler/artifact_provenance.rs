pub(crate) struct ArtifactProvenance;

impl ArtifactProvenance {
    pub(crate) const RUST_VERSION: &'static str = "1.97.1";
    const COMMIT: &'static str = "GITHUB_SHA";
    const COMMIT_LENGTHS: [usize; 2] = [40, 64];
    const SOURCE_DATE_EPOCH: &'static str = "SOURCE_DATE_EPOCH";

    pub(crate) fn commit() -> String {
        std::env::var(Self::COMMIT)
            .ok()
            .filter(|value| Self::is_commit_hash(value))
            .unwrap_or_else(|| "0".repeat(Self::COMMIT_LENGTHS[0]))
    }

    pub(crate) fn source_date_epoch() -> u64 {
        std::env::var(Self::SOURCE_DATE_EPOCH)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
    }

    fn is_commit_hash(value: &str) -> bool {
        [
            Self::COMMIT_LENGTHS.contains(&value.len()),
            value.bytes().all(|byte| byte.is_ascii_hexdigit()),
        ] == [true, true]
    }
}

#[cfg(test)]
mod tests {
    use super::ArtifactProvenance;

    #[test]
    fn only_a_full_hexadecimal_commit_is_a_commit() {
        assert!(ArtifactProvenance::is_commit_hash(&"a".repeat(40)));
        assert!(ArtifactProvenance::is_commit_hash(&"0f".repeat(32)));
        assert!(!ArtifactProvenance::is_commit_hash(&"g".repeat(40)));
        assert!(!ArtifactProvenance::is_commit_hash("abc123"));
    }

    #[test]
    fn fallback_revision_always_satisfies_the_manifest_shape() {
        let commit = ArtifactProvenance::commit();
        assert!(matches!(commit.len(), 40 | 64));
        assert!(commit.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
}
