use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

pub(crate) struct LuaProgram {
    folder: tempfile::TempDir,
}

impl LuaProgram {
    const PROGRAM: &'static str = "program.lua";

    pub(crate) fn new() -> Option<Self> {
        Self::available().then(|| Self {
            folder: tempfile::tempdir().unwrap_or_else(|error| panic!("{error}")),
        })
    }

    fn available() -> bool {
        let available = Command::new(super::LuaSource::RUNTIME)
            .arg("-v")
            .output()
            .is_ok_and(|output| output.status.success());
        if !available {
            eprintln!("skipping: lua is unavailable");
        }
        available
    }

    pub(crate) fn printed(parts: &[&str]) -> Option<String> {
        let program = Self::new()?;
        let path = program.file(Self::PROGRAM, parts);
        Some(Self::text(&program.run(&[path], &[], &[])))
    }

    pub(crate) fn file(&self, name: &str, parts: &[&str]) -> PathBuf {
        let path = self.folder.path().join(name);
        std::fs::write(&path, parts.concat()).unwrap_or_else(|error| panic!("{error}"));
        path
    }

    pub(crate) fn run(
        &self,
        files: &[PathBuf],
        environment: &[(&str, &str)],
        input: &[u8],
    ) -> Output {
        let mut child = Command::new(super::LuaSource::RUNTIME)
            .args(files)
            .envs(environment.iter().copied())
            .current_dir(self.folder.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("{error}"));
        let mut stdin = child
            .stdin
            .take()
            .unwrap_or_else(|| panic!("the Lua input is piped"));
        stdin
            .write_all(input)
            .unwrap_or_else(|error| panic!("{error}"));
        drop(stdin);
        child
            .wait_with_output()
            .unwrap_or_else(|error| panic!("{error}"))
    }

    pub(crate) fn frame(value: &serde_json::Value) -> Vec<u8> {
        let body = value.to_string().into_bytes();
        [format!("{:08x}", body.len()).into_bytes(), body].concat()
    }

    pub(crate) fn text(output: &Output) -> String {
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}
