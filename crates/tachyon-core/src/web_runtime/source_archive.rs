use super::WebRuntimeStep;
use crate::Failure;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

pub(crate) struct SourceArchive {
    pub(crate) name: String,
    pub(crate) version: String,
    checksum: String,
    urls: Vec<String>,
    step: WebRuntimeStep,
}

impl SourceArchive {
    const DOWNLOAD_LIMIT: Duration = Duration::from_mins(10);
    const CONNECT_SECONDS: &'static str = "15";
    const STALLED_BYTES_PER_SECOND: &'static str = "1024";
    const STALLED_SECONDS: &'static str = "60";
    const RETRIES: &'static str = "2";
    const RETRY_DELAY_SECONDS: &'static str = "2";
    const UNPACK_LIMIT: Duration = Duration::from_mins(5);

    pub(crate) fn pinned(step: WebRuntimeStep, sources: &str, name: &str) -> Result<Self, Failure> {
        let line = Self::line_named(sources, name)
            .ok_or_else(|| step.failure(&format!("The runtime sources do not pin '{name}'.")))?;
        Self::from_line(step, line)
    }

    pub(crate) async fn unpack_into(&self, work: &Path, target: &str) -> Result<(), Failure> {
        let archive = format!("{}.tar.gz", self.name);
        let url = self.download(&work.join(&archive)).await?;
        self.verify(&work.join(&archive), url)?;
        self.step.create_folder(&work.join(target))?;
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
        self.step
            .run(&mut command, &label, Self::UNPACK_LIMIT)
            .await
    }

    async fn download(&self, archive: &Path) -> Result<&str, Failure> {
        let mut refusals = Vec::with_capacity(self.urls.len());
        for url in &self.urls {
            let Err(failure) = self.download_from(url, archive).await else {
                return Ok(url);
            };
            refusals.push(failure.to_string());
        }
        Err(self.step.failure(&format!(
            "None of the {} pinned sources for {} answered. Tachyon tried them in order:\n{}",
            self.urls.len(),
            self.name,
            refusals.join("\n")
        )))
    }

    async fn download_from(&self, url: &str, archive: &Path) -> Result<(), Failure> {
        let mut command = Command::new("curl");
        command
            .args(["--fail", "--location", "--silent", "--show-error"])
            .args(["--connect-timeout", Self::CONNECT_SECONDS])
            .args(["--speed-limit", Self::STALLED_BYTES_PER_SECOND])
            .args(["--speed-time", Self::STALLED_SECONDS])
            .args(["--retry", Self::RETRIES, "--retry-all-errors"])
            .args(["--retry-delay", Self::RETRY_DELAY_SECONDS])
            .args(["--proto-redir", "=https", "--output"])
            .arg(archive)
            .arg(url);
        let label = format!("curl to download {} from {url}", self.name);
        self.step
            .run(&mut command, &label, Self::DOWNLOAD_LIMIT)
            .await
    }

    fn verify(&self, archive: &Path, url: &str) -> Result<(), Failure> {
        let bytes = self.step.read_file(archive)?;
        let actual = crate::compiler::hex_digest(Sha256::digest(&bytes));
        if actual == self.checksum {
            return Ok(());
        }
        let _ = std::fs::remove_file(archive);
        Err(self.step.failure(&format!(
            "The {} download from {} has SHA-256 {actual}, but Tachyon pins {}. Tachyon deleted the download and stopped.",
            self.name, url, self.checksum
        )))
    }

    fn line_named<'text>(sources: &'text str, name: &str) -> Option<&'text str> {
        sources
            .lines()
            .find(|line| line.split_whitespace().next() == Some(name))
    }

    fn from_line(step: WebRuntimeStep, line: &str) -> Result<Self, Failure> {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [name, version, checksum, first_url, ref fallback_urls @ ..] = fields[..] else {
            return Err(step.failure(&format!(
                "The runtime source line '{line}' needs a name, a version, a SHA-256 and at least one URL."
            )));
        };
        Ok(Self {
            name: String::from(name),
            version: String::from(version),
            checksum: String::from(checksum),
            urls: std::iter::once(first_url)
                .chain(fallback_urls.iter().copied())
                .map(String::from)
                .collect(),
            step,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{SourceArchive, WebRuntimeStep};
    use sha2::{Digest, Sha256};
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

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
        SourceArchive::pinned(
            WebRuntimeStep::new("Demo"),
            &format!("demo 1 {checksum} {url}\n"),
            "demo",
        )
        .expect("pinned")
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

    async fn refuse_once_then_serve(bytes: Vec<u8>) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let port = listener.local_addr().expect("address").port();
        let requests = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&requests);
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut request = vec![0_u8; 4096];
                let _read = stream.read(&mut request).await;
                let answer = if counted.fetch_add(1, Ordering::SeqCst) == 0 {
                    b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
                } else {
                    let mut answer = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    )
                    .into_bytes();
                    answer.extend_from_slice(&bytes);
                    answer
                };
                let _written = stream.write_all(&answer).await;
            }
        });
        (format!("http://127.0.0.1:{port}/demo.tar.gz"), requests)
    }

    #[tokio::test]
    async fn a_download_the_server_refuses_once_is_retried() {
        if !tools_available() {
            eprintln!("skipping: curl or tar is unavailable");
            return;
        }
        let work = tempfile::tempdir().expect("work");
        let bytes = std::fs::read(packed_folder(work.path())).expect("archive");
        let checksum = crate::compiler::hex_digest(Sha256::digest(&bytes));
        let (url, requests) = refuse_once_then_serve(bytes).await;
        SourceArchive::pinned(
            WebRuntimeStep::new("Demo"),
            &format!("demo 1 {checksum} {url}\n"),
            "demo",
        )
        .expect("pinned")
        .unpack_into(work.path(), "unpacked/demo")
        .await
        .expect("unpacked after one retry");
        let text =
            std::fs::read_to_string(work.path().join("unpacked/demo/hello.txt")).expect("file");
        assert_eq!(text, "hello");
        assert_eq!(requests.load(Ordering::SeqCst), 2);
    }

    async fn closed_url() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let port = listener.local_addr().expect("address").port();
        drop(listener);
        format!("http://127.0.0.1:{port}/demo.tar.gz")
    }

    #[tokio::test]
    async fn an_unreachable_source_falls_back_to_the_next_pinned_url() {
        if !tools_available() {
            eprintln!("skipping: curl or tar is unavailable");
            return;
        }
        let work = tempfile::tempdir().expect("work");
        let archive = packed_folder(work.path());
        let checksum =
            crate::compiler::hex_digest(Sha256::digest(std::fs::read(&archive).expect("archive")));
        let mirror = url::Url::from_file_path(&archive).expect("file url");
        let closed = closed_url().await;
        SourceArchive::pinned(
            WebRuntimeStep::new("Demo"),
            &format!("demo 1 {checksum} {closed} {mirror}\n"),
            "demo",
        )
        .expect("pinned")
        .unpack_into(work.path(), "unpacked/demo")
        .await
        .expect("unpacked from the mirror");
        let text =
            std::fs::read_to_string(work.path().join("unpacked/demo/hello.txt")).expect("file");
        assert_eq!(text, "hello");
    }

    #[tokio::test]
    async fn every_unreachable_source_is_named_in_the_failure() {
        if !tools_available() {
            eprintln!("skipping: curl or tar is unavailable");
            return;
        }
        let work = tempfile::tempdir().expect("work");
        let first = closed_url().await;
        let second = closed_url().await;
        let failure = SourceArchive::pinned(
            WebRuntimeStep::new("Demo"),
            &format!("demo 1 {} {first} {second}\n", "0".repeat(64)),
            "demo",
        )
        .expect("pinned")
        .unpack_into(work.path(), "unpacked/demo")
        .await
        .expect_err("no source answers");
        let message = failure.to_string();
        assert!(
            message.contains("None of the 2 pinned sources for demo answered"),
            "{message}"
        );
        assert!(message.contains(&first), "{message}");
        assert!(message.contains(&second), "{message}");
        assert!(!work.path().join("unpacked").exists());
    }

    #[test]
    fn a_source_line_needs_all_four_fields() {
        let failure = SourceArchive::pinned(WebRuntimeStep::new("Demo"), "demo 1 abc\n", "demo")
            .err()
            .expect("a short line must be refused");
        assert!(
            failure
                .to_string()
                .contains("needs a name, a version, a SHA-256 and at least one URL"),
            "{failure}"
        );
        assert!(
            SourceArchive::pinned(WebRuntimeStep::new("Demo"), "other 1 abc url\n", "demo")
                .is_err()
        );
    }
}
