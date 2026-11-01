use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

use super::FyloError;

pub(super) struct FyloLauncher {
    root: PathBuf,
}

pub(super) struct FyloProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl FyloLauncher {
    pub(super) async fn start(root: &Path) -> Result<(Self, FyloProcess), FyloError> {
        let mut process = FyloProcess::spawn(root)?;
        let identity = process.request(json!({ "op": "handshake" })).await?;
        Self::check_protocol(&identity)?;
        let launcher = Self {
            root: root.to_path_buf(),
        };
        Ok((launcher, process))
    }

    fn check_protocol(identity: &Value) -> Result<(), FyloError> {
        if identity["protocolVersion"] != json!(1) {
            return Err(FyloError::unsupported_protocol());
        }
        Ok(())
    }

    pub(super) fn launch(&self) -> Result<FyloProcess, FyloError> {
        FyloProcess::spawn(&self.root)
    }
}

impl FyloProcess {
    const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

    fn spawn(root: &Path) -> Result<Self, FyloError> {
        let mut child = tokio::process::Command::new("fylo")
            .args(["exec", "--loop", "--root"])
            .arg(root)
            .args(["--max-request-bytes", "1048576", "--max-response-bytes", "8388608"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                FyloError::protocol(
                    "The fylo binary is not on PATH; install FYLO (fylo.del.ma) to use this datasource.",
                )
            })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| FyloError::protocol("FYLO stdin is unavailable."))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| FyloError::protocol("FYLO stdout is unavailable."))?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub(super) async fn stop(mut self) {
        let _ = self.stdin.shutdown().await;
        let _ = self.child.kill().await;
    }

    pub(super) async fn request(&mut self, request: Value) -> Result<Value, FyloError> {
        let mut line = serde_json::to_string(&request)
            .map_err(|_| FyloError::protocol("FYLO request could not be encoded."))?;
        if line.len() > 1024 * 1024 {
            return Err(FyloError::protocol("FYLO request exceeds 1 MiB."));
        }
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|_| FyloError::protocol("FYLO subprocess is not accepting requests."))?;
        let mut answer = String::new();
        let mut reader =
            tokio::io::AsyncReadExt::take(&mut self.stdout, Self::MAX_RESPONSE_BYTES as u64 + 1);
        let read = reader
            .read_line(&mut answer)
            .await
            .map_err(|_| FyloError::protocol("FYLO subprocess closed its output."))?;
        if read == 0 {
            return Err(FyloError::protocol("FYLO subprocess exited."));
        }
        if answer.len() > Self::MAX_RESPONSE_BYTES {
            return Err(FyloError::protocol("FYLO response exceeds 8 MiB."));
        }
        let envelope: Value = serde_json::from_str(answer.trim_end())
            .map_err(|_| FyloError::protocol("FYLO returned a malformed frame."))?;
        if envelope["ok"] == json!(true) {
            return Ok(envelope["result"].clone());
        }
        Err(FyloError {
            code: envelope["error"]["code"]
                .as_str()
                .unwrap_or("EUNKNOWN")
                .to_owned(),
        })
    }
}
