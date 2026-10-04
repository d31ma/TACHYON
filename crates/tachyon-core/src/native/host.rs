use super::routes::NativeRouteIndex;
use crate::Failure;
use crate::external_command::run as supervise_tool;
use crate::failure::diagnostic;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

pub(super) struct NativeHost;

impl NativeHost {
    const MAX_SOURCE_BYTES: usize = 4 * 1_024 * 1_024;
    const MAX_TOOL_OUTPUT_BYTES: usize = 64 * 1_024;
    const MAX_BUNDLED_FILE_BYTES: u64 = 16 * 1_024 * 1_024;
    const TOOL_TIME_LIMIT: Duration = Duration::from_mins(2);
    const LONG_TOOL_TIME_LIMIT: Duration = Duration::from_mins(15);
    pub(super) const SHIM: &str = r"globalThis.__tachyonCompanionQueue = []
globalThis.__tachyonCompanionPublish = (signal) => {
  const queue = globalThis.__tachyonCompanionQueue
  if (queue.length === 128) queue.shift()
  queue.push(signal)
}
globalThis.__tachyonNativeHostCall = (capability, payload) =>
  globalThis.webkit?.messageHandlers?.tachyon
    ? globalThis.webkit.messageHandlers.tachyon.postMessage({ capability, payload })
    : globalThis.__tachyonHostPost?.(capability, payload)
globalThis.__tachyonWindowControls = Object.freeze(__WINDOW_CONTROLS__)
addEventListener('DOMContentLoaded', () => {
  if (document.body) document.body.dataset.platform = 'native'
})
";
}

pub(super) fn native_shim(window: &super::config::WindowConfiguration) -> String {
    let mut quoted = Vec::new();
    for control in &window.controls {
        quoted.push(format!("'{control}'"));
    }
    let granted = quoted.join(", ");
    NativeHost::SHIM.replace("__WINDOW_CONTROLS__", &format!("[{granted}]"))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct GeneratedHost {
    pub(super) application_bundle: PathBuf,
    pub(super) toolchain_name: String,
    pub(super) toolchain_version: String,
}

impl GeneratedHost {
    pub(super) fn not_packaged(application_bundle: PathBuf) -> Self {
        Self {
            application_bundle,
            toolchain_name: String::from("source"),
            toolchain_version: String::from("not-packaged"),
        }
    }
}

pub(super) fn stage_application(
    index: &NativeRouteIndex,
    web_bundle: &Path,
    stage: &Path,
    resources: &Path,
) -> Result<(), Failure> {
    for directory in [&stage.join("web"), &resources.to_path_buf()] {
        native_io(fs::create_dir_all(directory), directory)?;
    }

    let index_bytes = pretty_json(index, "Native route index")?;
    write(&stage.join("native-index.json"), &index_bytes)?;
    write(&resources.join("NativeIndex.json"), &index_bytes)?;

    copy_tree(web_bundle, &stage.join("web"))?;
    copy_tree(web_bundle, &resources.join("WebBundle"))?;
    let hidden_runtime = web_bundle.join(".tachyon");
    if hidden_runtime.is_dir() {
        copy_tree(&hidden_runtime, &stage.join("web/tachyon-runtime"))?;
        copy_tree(
            &hidden_runtime,
            &resources.join("WebBundle/tachyon-runtime"),
        )?;
    }

    Ok(())
}

pub(super) fn write_host_source(path: &Path, source: &str) -> Result<(), Failure> {
    if source.len() > NativeHost::MAX_SOURCE_BYTES {
        return Err(native_tool_failure(
            1605,
            "Generated native host exceeds the 4 MiB limit.",
        ));
    }
    write(path, source.as_bytes())
}

pub(super) async fn run_tool(program: &str, arguments: &[&str]) -> Result<String, Failure> {
    run_tool_in(program, arguments, None).await
}

pub(super) async fn run_tool_in(
    program: &str,
    arguments: &[&str],
    working_directory: Option<&Path>,
) -> Result<String, Failure> {
    run_tool_with(program, arguments, working_directory, &[]).await
}

pub(super) async fn run_long_tool(program: &str, arguments: &[&str]) -> Result<String, Failure> {
    let mut command = Command::new(program);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .kill_on_drop(true);
    supervised_output(program, &mut command, NativeHost::LONG_TOOL_TIME_LIMIT).await
}

pub(super) async fn run_tool_with(
    program: &str,
    arguments: &[&str],
    working_directory: Option<&Path>,
    environment: &[(&str, &str)],
) -> Result<String, Failure> {
    let mut command = Command::new(program);
    command
        .args(arguments)
        .envs(environment.iter().copied())
        .stdin(Stdio::null())
        .kill_on_drop(true);
    if let Some(working_directory) = working_directory {
        command.current_dir(working_directory);
    }
    supervised_output(program, &mut command, NativeHost::TOOL_TIME_LIMIT).await
}

async fn supervised_output(
    program: &str,
    command: &mut Command,
    time_limit: Duration,
) -> Result<String, Failure> {
    let output = supervise_tool(command, time_limit, NativeHost::MAX_TOOL_OUTPUT_BYTES)
        .await
        .map_err(|error| {
            native_tool_failure(
                1605,
                &format!("Cannot run native tool '{program}': {error}"),
            )
        })?;
    let stdout = bounded_text(&output.stdout);
    if !output.status.success() {
        let stderr = bounded_text(&output.stderr);
        return Err(failed_tool(program, output.status, &stdout, &stderr));
    }
    Ok(stdout)
}

fn failed_tool(
    program: &str,
    status: std::process::ExitStatus,
    stdout: &str,
    stderr: &str,
) -> Failure {
    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    native_tool_failure(
        1605,
        &format!("Native tool '{program}' failed with status {status}: {detail}"),
    )
}

pub(super) fn first_line(value: &str, fallback: &str) -> String {
    value
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(fallback)
        .trim()
        .chars()
        .take(64)
        .collect()
}

fn bounded_text(value: &[u8]) -> String {
    let length = value.len().min(NativeHost::MAX_TOOL_OUTPUT_BYTES);
    String::from_utf8_lossy(&value[..length]).into_owned()
}

pub(super) fn copy_tree(source: &Path, destination: &Path) -> Result<(), Failure> {
    native_io(fs::create_dir_all(destination), destination)?;
    let mut entries = native_io(
        fs::read_dir(source).and_then(Iterator::collect::<Result<Vec<_>, _>>),
        source,
    )?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let metadata = native_io(fs::symlink_metadata(&path), &path)?;
        if metadata.file_type().is_symlink() {
            return Err(native_tool_failure(
                1605,
                "Generated web bundle contains a symlink.",
            ));
        }
        let target = destination.join(entry.file_name());
        if metadata.is_dir() {
            copy_tree(&path, &target)?;
        } else if metadata.is_file() {
            refuse_oversized_file(&path, metadata.len())?;
            native_io(fs::copy(&path, &target), &target)?;
        }
    }
    Ok(())
}

fn refuse_oversized_file(path: &Path, size: u64) -> Result<(), Failure> {
    if size <= NativeHost::MAX_BUNDLED_FILE_BYTES {
        return Ok(());
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    Err(Failure::one(diagnostic(
        1605,
        format!("Web bundle file '{name}' is {size} bytes. Native hosts serve files up to 16 MiB."),
        Some(String::from(
            "Make the file smaller, or split it into several files.",
        )),
        None,
    )))
}

pub(super) fn write(path: &Path, contents: &[u8]) -> Result<(), Failure> {
    if let Some(parent) = path.parent() {
        native_io(fs::create_dir_all(parent), parent)?;
    }
    native_io(fs::write(path, contents), path)
}

pub(super) fn pretty_json(value: &impl serde::Serialize, label: &str) -> Result<Vec<u8>, Failure> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        native_tool_failure(1605, &format!("Cannot serialize {label}: {error}"))
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(super) fn native_io<T>(result: io::Result<T>, path: &Path) -> Result<T, Failure> {
    result.map_err(|error| {
        native_tool_failure(
            1605,
            &format!("Cannot write native artifact '{}': {error}", path.display()),
        )
    })
}

pub(super) fn native_tool_failure(number: u16, message: &str) -> Failure {
    Failure::one(diagnostic(
        number,
        message,
        Some(String::from(
            "Install the selected platform toolchain and keep the previous output intact.",
        )),
        None,
    ))
}

pub(super) fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(super) fn quoted_string_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\u{2028}', "\\u{2028}")
        .replace('\u{2029}', "\\u{2029}")
}

pub(super) fn c_string_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if (value as u32) < 0x20 => escaped.push(' '),
            value => escaped.push(value),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{
        NativeHost, c_string_escape, copy_tree, first_line, native_shim, quoted_string_escape,
        xml_escape,
    };
    use std::fs;

    #[test]
    fn generated_literals_are_escaped_without_changing_plain_names() {
        assert_eq!(quoted_string_escape("Native Catalog"), "Native Catalog");
        assert_eq!(
            quoted_string_escape("A \"quoted\" \\\u{2028} name"),
            "A \\\"quoted\\\" \\\\\\u{2028} name"
        );
        assert_eq!(c_string_escape("plain"), "plain");
        assert_eq!(c_string_escape("a\"b\\c\nd\u{1}e"), "a\\\"b\\\\c\\nd e");
        assert_eq!(xml_escape("a&b<c>"), "a&amp;b&lt;c&gt;");
    }

    #[test]
    fn version_banners_are_bounded_and_never_empty() {
        assert_eq!(first_line("\n\n  gcc 15.1  \nmore", "fallback"), "gcc 15.1");
        assert_eq!(first_line("   ", "fallback"), "fallback");
        assert_eq!(first_line(&"x".repeat(200), "fallback").len(), 64);
    }

    #[test]
    fn the_shim_gives_the_page_one_bridge_and_says_where_it_is_running() {
        assert!(NativeHost::SHIM.contains("__tachyonNativeHostCall"));
        assert!(NativeHost::SHIM.contains("messageHandlers.tachyon"));
        assert!(NativeHost::SHIM.contains("__tachyonHostPost"));
        assert!(NativeHost::SHIM.contains("dataset.platform = 'native'"));
        assert!(!NativeHost::SHIM.contains("tachyonWindow ="));
    }

    #[test]
    fn the_shim_gives_the_page_runtime_only_the_granted_window_controls() {
        let window = crate::native::config::WindowConfiguration {
            controls: vec![String::from("minimize"), String::from("title")],
            ..crate::native::config::WindowConfiguration::default()
        };
        let shim = native_shim(&window);
        assert!(
            shim.contains(
                "globalThis.__tachyonWindowControls = Object.freeze(['minimize', 'title'])"
            ),
            "{shim}"
        );
        let nothing = native_shim(&crate::native::config::WindowConfiguration::default());
        assert!(
            nothing.contains("globalThis.__tachyonWindowControls = Object.freeze([])"),
            "{nothing}"
        );
    }

    #[test]
    fn a_web_bundle_file_over_the_16_mib_limit_is_refused() {
        let bundle = tempfile::tempdir().expect("bundle");
        let module = fs::File::create(bundle.path().join("services.wasm")).expect("module");
        module
            .set_len(NativeHost::MAX_BUNDLED_FILE_BYTES)
            .expect("largest allowed size");
        let copied = tempfile::tempdir().expect("copy");
        copy_tree(bundle.path(), copied.path()).expect("a 16 MiB file is served");

        module
            .set_len(NativeHost::MAX_BUNDLED_FILE_BYTES + 1)
            .expect("one byte over");
        let error = copy_tree(bundle.path(), copied.path()).expect_err("too large");
        let message = error.to_string();
        assert!(message.contains("TY1605"), "{message}");
        assert!(message.contains("services.wasm"), "{message}");
        assert!(message.contains("16 MiB"), "{message}");
    }
}
