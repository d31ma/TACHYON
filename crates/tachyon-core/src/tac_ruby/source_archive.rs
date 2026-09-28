use super::runtime_step::RubyRuntimeStep;
use crate::Failure;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

pub(crate) struct SourceArchive {
    pub(crate) name: String,
    pub(crate) version: String,
    checksum: String,
    url: String,
}

impl SourceArchive {
    const DOWNLOAD_LIMIT: Duration = Duration::from_mins(10);
    const UNPACK_LIMIT: Duration = Duration::from_mins(5);

    pub(crate) fn pinned(sources: &str, name: &str) -> Result<Self, Failure> {
        let line = Self::line_named(sources, name).ok_or_else(|| {
            RubyRuntimeStep::failure(&format!("The Ruby runtime sources do not pin '{name}'."))
        })?;
        Self::from_line(line)
    }

    pub(crate) async fn unpack_into(&self, work: &Path, target: &str) -> Result<(), Failure> {
        let archive = format!("{}.tar.gz", self.name);
        self.download(&work.join(&archive)).await?;
        self.verify(&work.join(&archive))?;
        RubyRuntimeStep::create_folder(&work.join(target))?;
        let mut command = Command::new("tar");
        command
            .args([
                "-xzf",
                archive.as_str(),
                "-C",
                target,
                "--strip-components=1",
            ])
            .current_dir(work);
        let label = format!("tar to unpack {}", self.name);
        RubyRuntimeStep::run(&mut command, &label, Self::UNPACK_LIMIT).await
    }

    async fn download(&self, archive: &Path) -> Result<(), Failure> {
        let mut command = Command::new("curl");
        command
            .args(["--fail", "--location", "--silent", "--show-error"])
            .args(["--proto-redir", "=https", "--output"])
            .arg(archive)
            .arg(&self.url);
        let label = format!("curl to download {}", self.name);
        RubyRuntimeStep::run(&mut command, &label, Self::DOWNLOAD_LIMIT).await
    }

    fn verify(&self, archive: &Path) -> Result<(), Failure> {
        let bytes = RubyRuntimeStep::read_file(archive)?;
        let actual = crate::compiler::hex_digest(Sha256::digest(&bytes));
        if actual == self.checksum {
            return Ok(());
        }
        let _ = std::fs::remove_file(archive);
        Err(RubyRuntimeStep::failure(&format!(
            "The {} download from {} has SHA-256 {actual}, but Tachyon pins {}. Tachyon deleted the download and stopped.",
            self.name, self.url, self.checksum
        )))
    }

    fn line_named<'text>(sources: &'text str, name: &str) -> Option<&'text str> {
        sources
            .lines()
            .find(|line| line.split_whitespace().next() == Some(name))
    }

    fn from_line(line: &str) -> Result<Self, Failure> {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [name, version, checksum, url] = fields[..] else {
            return Err(RubyRuntimeStep::failure(&format!(
                "The Ruby runtime source line '{line}' needs a name, a version, a SHA-256 and a URL."
            )));
        };
        Ok(Self {
            name: String::from(name),
            version: String::from(version),
            checksum: String::from(checksum),
            url: String::from(url),
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::SourceArchive;
    use sha2::{Digest, Sha256};
    use std::path::Path;

    fn tools_available() -> bool {
        ["curl", "tar"].iter().all(|tool| {
            std::process::Command::new(tool)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
        })
    }

    fn packed_folder(work: &Path) -> std::path::PathBuf {
        let folder = work.join("upstream/demo-1");
        std::fs::create_dir_all(&folder).expect("folder");
        std::fs::write(folder.join("hello.txt"), "hello").expect("file");
        let archive = work.join("upstream/demo.tar.gz");
        let status = std::process::Command::new("tar")
            .args(["-czf", "demo.tar.gz", "demo-1"])
            .current_dir(work.join("upstream"))
            .status()
            .expect("tar");
        assert!(status.success());
        archive
    }

    fn pinned(archive: &Path, checksum: &str) -> SourceArchive {
        let url = url::Url::from_file_path(archive).expect("file url");
        SourceArchive::pinned(&format!("demo 1 {checksum} {url}\n"), "demo").expect("pinned")
    }

    #[tokio::test]
    async fn an_archive_with_the_pinned_hash_is_unpacked() {
        if !tools_available() {
            eprintln!("skipping: curl or tar is unavailable");
            return;
        }
        let work = tempfile::tempdir().expect("work");
        let archive = packed_folder(work.path());
        let checksum =
            crate::compiler::hex_digest(Sha256::digest(std::fs::read(&archive).expect("archive")));
        pinned(&archive, &checksum)
            .unpack_into(work.path(), "unpacked/demo")
            .await
            .expect("unpacked");
        let text =
            std::fs::read_to_string(work.path().join("unpacked/demo/hello.txt")).expect("file");
        assert_eq!(text, "hello");
    }

    #[tokio::test]
    async fn an_archive_with_another_hash_is_deleted_and_never_unpacked() {
        if !tools_available() {
            eprintln!("skipping: curl or tar is unavailable");
            return;
        }
        let work = tempfile::tempdir().expect("work");
        let archive = packed_folder(work.path());
        let failure = pinned(&archive, &"0".repeat(64))
            .unpack_into(work.path(), "unpacked/demo")
            .await
            .expect_err("a changed archive must be refused");
        assert!(
            failure.to_string().contains("but Tachyon pins 0000"),
            "{failure}"
        );
        assert!(!work.path().join("demo.tar.gz").exists());
        assert!(!work.path().join("unpacked").exists());
    }

    #[test]
    fn a_source_line_needs_all_four_fields() {
        let failure = SourceArchive::pinned("demo 1 abc\n", "demo")
            .err()
            .expect("a short line must be refused");
        assert!(
            failure
                .to_string()
                .contains("needs a name, a version, a SHA-256 and a URL"),
            "{failure}"
        );
        assert!(SourceArchive::pinned("other 1 abc url\n", "demo").is_err());
    }
}
