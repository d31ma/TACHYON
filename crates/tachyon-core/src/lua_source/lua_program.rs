use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

pub(crate) struct LuaProgram {
    folder: tempfile::TempDir,
}

impl LuaProgram {
    const PROGRAM: &'static str = "program.lua";

    pub(crate) fn new() -> Option<Self> {
        let available = Command::new(super::LuaSource::RUNTIME)
            .arg("-v")
            .output()
            .is_ok_and(|output| output.status.success());
        if !available {
            eprintln!("skipping: lua is unavailable");
            return None;
        }
        let folder = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        Some(Self { folder })
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
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input)
                .unwrap_or_else(|error| panic!("{error}"));
        }
        child
            .wait_with_output()
            .unwrap_or_else(|error| panic!("{error}"))
    }

    pub(crate) fn frame(value: &serde_json::Value) -> Vec<u8> {
        let body = value.to_string().into_bytes();
        let length = u32::try_from(body.len()).unwrap_or_else(|error| panic!("{error}"));
        [length.to_be_bytes().to_vec(), body].concat()
    }

    pub(crate) fn text(output: &Output) -> String {
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}
