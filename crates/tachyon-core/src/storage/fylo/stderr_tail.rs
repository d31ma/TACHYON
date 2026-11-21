use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};
use tokio::task::JoinHandle;

use crate::sorted_json::SortedJson;

pub(super) struct StderrTail {
    lines: Arc<Mutex<VecDeque<String>>>,
    reader: JoinHandle<()>,
    root: String,
}

impl StderrTail {
    const EVENT: &'static str = "storage.fylo_stderr";
    const ROOT: &'static str = "<root>";
    const MAX_LINES: usize = 20;
    const MAX_BYTES: usize = 4 * 1024;
    const MAX_LINE_BYTES: usize = 512;
    const LONGEST_READ: u64 = 8 * 1024;
    const READ_SETTLE: Duration = Duration::from_millis(200);

    pub(super) fn follow<Output>(stderr: Output, root: &Path) -> Self
    where
        Output: AsyncRead + Unpin + Send + 'static,
    {
        let lines = Arc::new(Mutex::new(VecDeque::new()));
        let reader = tokio::spawn(Self::read(stderr, Arc::clone(&lines)));
        Self {
            lines,
            reader,
            root: root.display().to_string(),
        }
    }

    pub(super) async fn report(&mut self, operation: &str) {
        let _finished_or_still_reading =
            tokio::time::timeout(Self::READ_SETTLE, &mut self.reader).await;
        Self::print(self.event(operation));
    }

    fn print(event: Option<Value>) {
        if let Some(event) = event {
            eprintln!("{}", SortedJson::text(&event));
        }
    }

    fn event(&self, operation: &str) -> Option<Value> {
        let lines: Vec<String> = Self::locked(&self.lines)
            .iter()
            .map(|line| line.replace(&self.root, Self::ROOT))
            .collect();
        let written = !lines.is_empty();
        written.then(|| json!({"event": Self::EVENT, "operation": operation, "lines": lines}))
    }

    async fn read<Output>(stderr: Output, lines: Arc<Mutex<VecDeque<String>>>)
    where
        Output: AsyncRead + Unpin,
    {
        let mut reader = BufReader::new(stderr);
        let mut line = Vec::new();
        while Self::next_line(&mut reader, &mut line).await {
            Self::keep(&mut Self::locked(&lines), &line);
        }
    }

    async fn next_line<Output>(reader: &mut BufReader<Output>, line: &mut Vec<u8>) -> bool
    where
        Output: AsyncRead + Unpin,
    {
        line.clear();
        let mut limited = AsyncReadExt::take(reader, Self::LONGEST_READ);
        limited
            .read_until(b'\n', line)
            .await
            .is_ok_and(|read| read > 0)
    }

    fn keep(kept: &mut VecDeque<String>, line: &[u8]) {
        kept.push_back(Self::cut(line));
        while Self::too_big(kept) {
            kept.pop_front();
        }
    }

    fn cut(line: &[u8]) -> String {
        let end = line.len().min(Self::MAX_LINE_BYTES);
        String::from_utf8_lossy(&line[..end]).trim_end().to_owned()
    }

    fn too_big(kept: &VecDeque<String>) -> bool {
        let bytes: usize = kept.iter().map(String::len).sum();
        kept.len() > Self::MAX_LINES || bytes > Self::MAX_BYTES
    }

    fn locked(lines: &Mutex<VecDeque<String>>) -> MutexGuard<'_, VecDeque<String>> {
        lines.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::StderrTail;

    async fn tail_of(written: String) -> StderrTail {
        let mut tail = StderrTail::follow(
            std::io::Cursor::new(written.into_bytes()),
            Path::new("/srv/fylo-root"),
        );
        let _done = (&mut tail.reader).await;
        tail
    }

    #[tokio::test]
    async fn only_the_last_twenty_lines_are_kept() {
        let lines: Vec<String> = (1..=30).map(|line| format!("line {line}")).collect();
        let written = lines.join("\n") + "\n";
        let tail = tail_of(written).await;
        let event = tail.event("putData").unwrap_or_default();
        let lines = event["lines"].as_array().cloned().unwrap_or_default();
        assert_eq!(lines.len(), 20);
        assert_eq!(lines[0], json!("line 11"));
        assert_eq!(lines[19], json!("line 30"));
    }

    #[tokio::test]
    async fn long_lines_are_cut_and_the_tail_stays_under_four_kibibytes() {
        let written = format!("{}\n", "x".repeat(2_000)).repeat(15);
        let tail = tail_of(written).await;
        let event = tail.event("putData").unwrap_or_default();
        let lines = event["lines"].as_array().cloned().unwrap_or_default();
        let bytes: usize = lines
            .iter()
            .filter_map(|line| line.as_str())
            .map(str::len)
            .sum();
        assert!(
            lines
                .iter()
                .all(|line| line.as_str().is_some_and(|text| text.len() <= 512))
        );
        assert!(bytes <= 4 * 1024, "{bytes}");
        assert!(!lines.is_empty());
    }

    #[tokio::test]
    async fn the_root_path_is_replaced() {
        let written = String::from("error: cannot open /srv/fylo-root/people/index (pid 4242)\n");
        let tail = tail_of(written).await;
        let event = tail.event("getDoc").unwrap_or_default();
        assert_eq!(
            event,
            json!({
                "event": "storage.fylo_stderr",
                "operation": "getDoc",
                "lines": ["error: cannot open <root>/people/index (pid 4242)"],
            })
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_killed_process_leaves_its_last_lines() {
        let mut child = tokio::process::Command::new("sh")
            .args([
                "-c",
                "echo 'open /srv/fylo-root/people failed' >&2; exec sleep 30",
            ])
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap_or_else(|error| panic!("{error}"));
        let stderr = child.stderr.take().unwrap_or_else(|| panic!("stderr"));
        let tail = StderrTail::follow(stderr, Path::new("/srv/fylo-root"));
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let _killed = child.kill().await;
        let event = tail.event("putData").unwrap_or_default();
        assert_eq!(event["lines"], json!(["open <root>/people failed"]));
    }

    #[tokio::test]
    async fn a_silent_fylo_writes_no_line() {
        let tail = tail_of(String::new()).await;
        assert_eq!(tail.event("putData"), None);
    }
}
