#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use super::windows::WindowsHostGenerator;
use crate::project::NativeCompanion;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

struct NativeTaskFixture;

impl NativeTaskFixture {
    const SERVICE: &'static str =
        include_str!("../../tests/fixtures/native-csharp-tasks/NotesService.cs");
    const CLIENT: &'static str =
        include_str!("../../tests/fixtures/native-csharp-tasks/NotesClient.cs");
    const SETUP: &'static str =
        include_str!("../../tests/fixtures/native-csharp-tasks/NotesSetup.cs");
    const DRIVER: &'static str =
        include_str!("../../tests/fixtures/native-csharp-tasks/NativeTaskDriver.cs");
    const PROJECT: &'static str = "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net9.0</TargetFramework><OutputType>Exe</OutputType><Nullable>disable</Nullable><AllowUnsafeBlocks>true</AllowUnsafeBlocks><InvariantGlobalization>true</InvariantGlobalization><EnableDefaultCompileItems>false</EnableDefaultCompileItems></PropertyGroup><ItemGroup><Compile Include=\"TachyonCompanion.cs\" /><Compile Include=\"NativeTaskDriver.cs\" /></ItemGroup></Project>";

    fn write(root: &Path, relative: &str, source: &str) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture directory");
        std::fs::write(&path, source).expect("fixture source");
        path
    }

    fn companions(root: &Path) -> Vec<NativeCompanionInput> {
        Self::write(root, "client/remotes/NotesClient.cs", Self::CLIENT);
        Self::write(root, "client/setups/NotesSetup.cs", Self::SETUP);
        let source = Self::write(root, "client/services/NotesService.cs", Self::SERVICE);
        vec![NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::CSharp,
            source,
            service: String::from("notes"),
            class: String::from("NotesService"),
            web_assembly: false,
            application_provider: false,
        }]
    }

    fn staged(root: &Path, origin: Option<&str>) -> PathBuf {
        let companions = Self::companions(root);
        WindowsHostGenerator::staged_companion(&companions, root, ("dev.tachyon.notes", origin))
            .expect("stage")
            .expect("C# project")
    }

    fn source(origin: Option<&str>) -> String {
        let root = tempfile::tempdir().expect("project");
        let project = Self::staged(root.path(), origin);
        std::fs::read_to_string(project.join("TachyonCompanion.cs")).expect("companion source")
    }

    fn available() -> bool {
        std::process::Command::new("dotnet")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn run(origin: &str) -> String {
        let root = tempfile::tempdir().expect("project");
        let project = Self::staged(root.path(), Some(origin));
        std::fs::write(project.join("TachyonCompanion.csproj"), Self::PROJECT)
            .expect("project file");
        std::fs::write(project.join("NativeTaskDriver.cs"), Self::DRIVER).expect("driver");
        let mut command = std::process::Command::new("dotnet");
        command.arg("run").arg("--project").arg(&project);
        let output =
            crate::external_command::run_sync(&mut command, Duration::from_mins(4), 65_536)
                .expect("supervised .NET native task fixture");
        let printed = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "{printed}");
        printed
    }
}

struct NotesYon;

impl NotesYon {
    const LARGE_TEXT_LENGTH: usize = 70_000;
    const SLOW_ANSWER: Duration = Duration::from_secs(30);

    fn start() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("local Yon");
        let origin = format!("http://{}", listener.local_addr().expect("local address"));
        std::thread::spawn(move || Self::serve(&listener));
        origin
    }

    fn serve(listener: &TcpListener) {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || Self::answer(stream));
        }
    }

    fn answer(mut stream: TcpStream) {
        let mut reader = BufReader::new(stream.try_clone().expect("stream"));
        let (target, headers) = Self::read_head(&mut reader);
        let body = Self::read_body(&mut reader, &headers);
        let (status, extra, text) = Self::route(&target, &headers, &body);
        let response = format!(
            "HTTP/1.1 {status} Answer\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{text}",
            text.len()
        );
        let _ = stream.write_all(response.as_bytes());
    }

    fn read_head(reader: &mut BufReader<TcpStream>) -> (String, Vec<(String, String)>) {
        let mut first = String::new();
        let _ = reader.read_line(&mut first);
        let target = first.split(' ').nth(1).unwrap_or("/").to_owned();
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            let Some((name, value)) = line.trim_end().split_once(':') else {
                return (target, headers);
            };
            headers.push((name.to_ascii_lowercase(), value.trim().to_owned()));
        }
    }

    fn header(headers: &[(String, String)], name: &str) -> String {
        headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    }

    fn read_body(reader: &mut BufReader<TcpStream>, headers: &[(String, String)]) -> String {
        let length = Self::header(headers, "content-length")
            .parse::<usize>()
            .unwrap_or(0);
        let mut body = vec![0; length];
        let _ = reader.read_exact(&mut body);
        String::from_utf8_lossy(&body).into_owned()
    }

    fn route(target: &str, headers: &[(String, String)], body: &str) -> (u16, String, String) {
        let path = target.split('?').next().unwrap_or(target);
        match path {
            "/api/notes/first" => (200, String::new(), String::from("{\"title\":\"first\"}")),
            "/api/notes/missing" => (404, String::new(), String::from("{\"error\":\"missing\"}")),
            "/api/notes/moved" => (
                302,
                String::from("Location: /api/notes/first\r\n"),
                String::new(),
            ),
            "/api/notes/empty" => (200, String::new(), String::new()),
            "/api/notes/broken" => (200, String::new(), String::from("{not json")),
            "/api/notes/unsafe" => (200, String::new(), String::from("{\"n\":9007199254740993}")),
            "/api/notes/large" => (200, String::new(), Self::large()),
            "/api/notes/echo" => (200, String::new(), Self::echo(target, headers, body)),
            _ => Self::slow(),
        }
    }

    fn large() -> String {
        format!("{{\"text\":\"{}\"}}", "x".repeat(Self::LARGE_TEXT_LENGTH))
    }

    fn echo(target: &str, headers: &[(String, String)], body: &str) -> String {
        serde_json::json!({
            "target": target,
            "type": Self::header(headers, "content-type"),
            "note": Self::header(headers, "x-note"),
            "cookie": Self::header(headers, "cookie"),
            "body": body,
        })
        .to_string()
    }

    fn slow() -> (u16, String, String) {
        std::thread::sleep(Self::SLOW_ANSWER);
        (200, String::new(), String::from("{\"title\":\"late\"}"))
    }
}

#[test]
fn the_staged_csharp_companion_starts_tasks_and_reaches_the_declared_yon_origin() {
    let source = NativeTaskFixture::source(Some("https://api.example.com"));
    for expected in [
        "internal const string YonOrigin = \"https://api.example.com\";",
        "[System.Runtime.CompilerServices.ModuleInitializer]",
        "TacHttpTransport.Start ??= TacNativeHttp.Start;",
        "TacHttpTransport.Answer ??= TacNativeHttp.Answer;",
        "TacHttpTransport.Cancel ??= TacNativeHttp.Cancel;",
        "TachyonService0.BehaviorBrowserHttp.Capture ??= Tac.CaptureHttpTransport;",
        "TachyonService0.BehaviorError failure0 => failure0.Code",
        "Marshal.StringToCoTaskMemUTF8(Tac.NativeInvoke(text ?? \"{}\"))",
        "TacOwnerContext.Run(OwnerLock, () => InvokeAsync(request))",
        "[\"load\"] = TacMember.TaskMethod(async arguments =>",
        "[\"total\"] = TacMember.Method(",
        "var Instance = new NotesService(__tacClient0)",
        "AllowAutoRedirect = false",
        "UseCookies = false",
        "AutomaticDecompression = System.Net.DecompressionMethods.None",
    ] {
        assert!(source.contains(expected), "{expected}");
    }
    assert!(!source.contains("__TACHYON_NATIVE_BINDINGS__"));
    assert!(!source.contains("GetAwaiter().GetResult()"));
    assert_eq!(source.matches("public static class TacNative\n").count(), 1);
    assert_eq!(
        source
            .matches("internal static class TacHttpTransport")
            .count(),
        1
    );
}

#[test]
fn a_staged_csharp_companion_without_a_yon_origin_answers_http_missing() {
    let source = NativeTaskFixture::source(None);
    assert!(source.contains("internal const string YonOrigin = \"\";"));
    assert!(source.contains(
        "if (TacApplication.YonOrigin.Length == 0) return TacNativeHttpFailure.Answer(\"HTTP_MISSING\");"
    ));
}

#[test]
fn native_csharp_tasks_answer_a_task_number_and_signal_their_yon_answer_once() {
    if !NativeTaskFixture::available() {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    let origin = NotesYon::start();
    let output = NativeTaskFixture::run(&origin);
    println!("{output}");
    assert!(
        output.contains("PASS native C# tasks reach Yon"),
        "{output}"
    );
}
