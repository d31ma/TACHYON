#![allow(clippy::expect_used)]

use super::native_page_graph_tests::NativePageGraphCheck;
use super::registry::NativeCompanionInput;
use super::windows_host_tests::WindowsHostTests;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) struct NativeRustTaskCheck;

impl NativeRustTaskCheck {
    const ORIGIN: &'static str = "https://yon.example.com";
    const WAITING: &'static str = r#"
#[Service]
#[derive(Default)]
struct WaitService {
    count: i64,
}

impl WaitService {
    pub async fn later(&mut self) -> String {
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut started = false;
        std::future::poll_fn(|context| {
            if done.load(std::sync::atomic::Ordering::SeqCst) {
                return std::task::Poll::Ready(());
            }
            if !started {
                started = true;
                let waker = context.waker().clone();
                let flag = done.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    waker.wake();
                });
            }
            std::task::Poll::Pending
        })
        .await;
        self.count += 1;
        String::from("later")
    }
}
"#;
    const WAITING_CALLER: &'static str = r##"
struct WaitProbe;

impl WaitProbe {
    const OWNER: &'static str = "00000000-0000-4000-8000-000000000081";

    fn signals() -> &'static std::sync::Mutex<Vec<String>> {
        static SIGNALS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        &SIGNALS
    }

    fn dispatched() -> &'static std::sync::atomic::AtomicUsize {
        static DISPATCHED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        &DISPATCHED
    }

    extern "C" fn emit(payload: *const core::ffi::c_char) {
        let text = unsafe { std::ffi::CStr::from_ptr(payload) }.to_string_lossy().into_owned();
        Self::signals().lock().expect("signals").push(text);
    }

    extern "C" fn dispatch() {
        Self::dispatched().fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn request(operation: &str, name: &str) -> String {
        tac_handle(&format!(
            r#"{{"route":"/","service":"root","owner":"{}","op":"{operation}","name":"{name}","args":[]}}"#,
            Self::OWNER
        ))
    }

    fn wait_for_dispatch() {
        for _ in 0..200 {
            if Self::dispatched().load(std::sync::atomic::Ordering::SeqCst) > 0 {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the waker never asked the host to resume");
    }

    fn resume_until_answered() {
        for _ in 0..200 {
            assert_eq!(tac_handle(r#"{"op":"host-resume"}"#), r#"{"value":null}"#);
            if !Self::signals().lock().expect("signals").is_empty() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the resumed task never answered");
    }
}

fn main() {
    let init = WaitProbe::request("init", "");
    assert!(!init.contains("error"), "{init}");
    let refused = WaitProbe::request("call", "later");
    assert!(refused.contains("HTTP_MISSING"), "a host without a dispatch hook keeps refusing: {refused}");
    unsafe { tac_native_set_emit(Some(WaitProbe::emit)) };
    unsafe { TacTaskDispatch::tac_native_set_dispatch(Some(WaitProbe::dispatch)) };
    let started = WaitProbe::request("call", "later");
    assert_eq!(started, r#"{"task":2}"#, "the refused call spent task number 1");
    WaitProbe::wait_for_dispatch();
    assert!(WaitProbe::signals().lock().expect("signals").is_empty(), "nothing finishes before the host resumes");
    WaitProbe::resume_until_answered();
    let signals = WaitProbe::signals().lock().expect("signals").clone();
    assert_eq!(
        signals,
        vec![format!(
            r#"{{"route":"/","service":"root","owner":"{}","task":2,"answer":"{{\"value\":\"later\"}}"}}"#,
            WaitProbe::OWNER
        )]
    );
    assert_eq!(WaitProbe::request("get", "count"), r#"{"value":1}"#);
    let second = WaitProbe::request("call", "later");
    assert_eq!(second, r#"{"task":3}"#);
    assert!(WaitProbe::request("dispose", "").contains("null"));
    let after = WaitProbe::signals().lock().expect("signals").clone();
    assert_eq!(after.len(), 2, "a disposed owner's task is cancelled once: {after:?}");
    assert!(after[1].contains(r#""task":3,"#) && after[1].contains("CANCELLED"), "{after:?}");
    println!("native Rust tasks wait for the host dispatch hook");
}
"##;
    const PING: &'static str = "pub struct Ping {\n    pub step: i64,\n}\n";
    const LISTENING: &'static str = r#"
use crate::models::Ping;

#[derive(Default)]
#[Service]
struct ListenService {
    heard: String,
    application_event_publisher: ApplicationEventPublisher,
}

impl ListenService {
    pub fn send(&mut self, step: i64) {
        self.application_event_publisher.publish_event(Ping { step }, false);
    }

    #[OnEvent]
    async fn later(&mut self, value: Ping) {
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut started = false;
        std::future::poll_fn(|context| {
            if done.load(std::sync::atomic::Ordering::SeqCst) {
                return std::task::Poll::Ready(());
            }
            if !started {
                started = true;
                let waker = context.waker().clone();
                let flag = done.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    waker.wake();
                });
            }
            std::task::Poll::Pending
        })
        .await;
        self.heard.push_str(&format!("a{}", value.step));
    }

    #[OnEvent]
    fn now(&mut self, value: Ping) {
        self.heard.push_str(&format!("s{}", value.step));
    }
}
"#;
    const LISTENING_CALLER: &'static str = r##"
struct ListenProbe;

impl ListenProbe {
    const OWNER: &'static str = "00000000-0000-4000-8000-000000000083";

    fn signals() -> &'static std::sync::Mutex<Vec<String>> {
        static SIGNALS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        &SIGNALS
    }

    extern "C" fn emit(payload: *const core::ffi::c_char) {
        let text = unsafe { std::ffi::CStr::from_ptr(payload) }.to_string_lossy().into_owned();
        Self::signals().lock().expect("signals").push(text);
    }

    extern "C" fn dispatch() {}

    fn request(operation: &str, name: &str, arguments: &str) -> String {
        tac_invoke_owned(&format!(
            r#"{{"route":"/","service":"root","owner":"{}","op":"{operation}","name":"{name}","args":[{arguments}]}}"#,
            Self::OWNER
        ))
    }

    fn resume_until_heard(wanted: &str) {
        for _ in 0..300 {
            assert_eq!(tac_handle(r#"{"op":"host-resume"}"#), r#"{"value":null}"#);
            if Self::request("get", "heard", "") == format!(r#"{{"value":"{wanted}"}}"#) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the listeners never heard {wanted}: {}", Self::request("get", "heard", ""));
    }
}

fn main() {
    unsafe { tac_native_set_emit(Some(ListenProbe::emit)) };
    unsafe { TacTaskDispatch::tac_native_set_dispatch(Some(ListenProbe::dispatch)) };
    let init = ListenProbe::request("init", "", "");
    assert!(!init.contains("error"), "{init}");
    assert_eq!(ListenProbe::request("call", "send", "1"), r#"{"value":null}"#);
    assert!(ListenProbe::request("call", "send", "2").contains(r#""code":"BUSY""#), "a call waits while the listener task holds the service");
    assert!(ListenProbe::request("get", "heard", "").contains(r#""code":"BUSY""#), "a field read waits while the listener task holds the service");
    ListenProbe::resume_until_heard("a1s1");
    assert_eq!(ListenProbe::request("call", "send", "2"), r#"{"value":null}"#);
    ListenProbe::resume_until_heard("a1s1a2s2");
    let signals = ListenProbe::signals().lock().expect("signals").clone();
    let pings = signals.iter().filter(|signal| signal.contains(r#""name":"Ping""#)).count();
    let changed = signals.iter().filter(|signal| signal.contains(r#""changed":true"#)).count();
    assert_eq!([pings, changed, signals.len()], [2, 2, 4], "each finished listener task tells the page to read the service again: {signals:?}");
    assert!(signals.iter().all(|signal| !signal.contains(r#""task":"#)), "{signals:?}");
    assert_eq!(ListenProbe::request("call", "send", "3"), r#"{"value":null}"#);
    assert!(ListenProbe::request("dispose", "", "").contains("null"));
    let after = ListenProbe::signals().lock().expect("signals").clone();
    assert!(after.iter().all(|signal| !signal.contains(r#""task":"#)), "a cancelled listener task sends no task answer: {after:?}");
    println!("native Rust async listeners run one after another through the host dispatch hook");
}
"##;
    const GREETING: &'static str = "pub struct Greeting {\n    pub message: String,\n}\n";
    const CLIENT: &'static str = "use crate::models::Greeting;\n\n#[HTTPClient(\"/api\")]\ntrait GreetingClient {\n    #[GetRequest(\"/greeting\")]\n    async fn get_greeting(&self) -> BehaviorResult<Greeting>;\n    #[GetRequest(\"/refused\")]\n    async fn get_refused(&self) -> BehaviorResult<Greeting>;\n}\n";
    const SETUP: &'static str = "#[Setup]\nstruct GreetingSetup {}\nimpl GreetingSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n";
    const ASKING: &'static str = "#[Service]\npub struct CounterService {\n    #[Needs]\n    greeting_client: std::sync::Arc<GreetingClient>,\n    pub message: String,\n}\n\nimpl CounterService {\n    pub async fn ask_server(&mut self) -> BehaviorResult<String> {\n        self.message = self.greeting_client.get_greeting().await?.message;\n        Ok(self.message.clone())\n    }\n\n    pub async fn ask_refused(&mut self) -> BehaviorResult<String> {\n        Ok(self.greeting_client.get_refused().await?.message)\n    }\n}\n";
    const ASKING_CALLER: &'static str = r##"
fn main() {
    let owner = "00000000-0000-4000-8000-000000000082";
    let request = |operation: &str| tac_handle(&format!(
        r#"{{"route":"/","service":"root","owner":"{owner}","op":"{operation}","name":"askServer","args":[]}}"#
    ));
    let init = request("init");
    assert!(!init.contains("error"), "{init}");
    let answer = request("call");
    assert!(answer.contains("HTTP_MISSING"), "a companion without an HTTP sender for its platform answers HTTP_MISSING: {answer}");
    assert_eq!(TAC_YON_ORIGIN, "https://yon.example.com");
    println!("native Rust HTTP clients answer HTTP_MISSING without a platform sender");
}
"##;
    const REACHING_CALLER: &'static str = r##"
struct ReachProbe;

impl ReachProbe {
    const OWNER: &'static str = "00000000-0000-4000-8000-000000000083";

    fn signals() -> &'static std::sync::Mutex<Vec<String>> {
        static SIGNALS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        &SIGNALS
    }

    fn dispatched() -> &'static std::sync::atomic::AtomicUsize {
        static DISPATCHED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        &DISPATCHED
    }

    extern "C" fn emit(payload: *const core::ffi::c_char) {
        let text = unsafe { std::ffi::CStr::from_ptr(payload) }.to_string_lossy().into_owned();
        Self::signals().lock().expect("signals").push(text);
    }

    extern "C" fn dispatch() {
        Self::dispatched().fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn request(operation: &str, name: &str) -> String {
        tac_handle(&format!(
            r#"{{"route":"/","service":"root","owner":"{}","op":"{operation}","name":"{name}","args":[]}}"#,
            Self::OWNER
        ))
    }

    fn ask(name: &str) -> String {
        let heard = Self::signals().lock().expect("signals").len();
        let started = Self::request("call", name);
        assert!(started.starts_with(r#"{"task":"#), "{started}");
        for _ in 0..3000 {
            if Self::signals().lock().expect("signals").len() > heard {
                break;
            }
            if Self::dispatched().swap(0, std::sync::atomic::Ordering::SeqCst) > 0 {
                tac_handle(r#"{"op":"host-resume"}"#);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Self::signals().lock().expect("signals").get(heard).cloned().expect("the task never answered")
    }
}

fn main() {
    let init = ReachProbe::request("init", "");
    assert!(!init.contains("error"), "{init}");
    unsafe { tac_native_set_emit(Some(ReachProbe::emit)) };
    unsafe { TacTaskDispatch::tac_native_set_dispatch(Some(ReachProbe::dispatch)) };
    println!("{}", ReachProbe::ask("askServer"));
    println!("{}", ReachProbe::ask("askRefused"));
    println!("{}", ReachProbe::request("get", "message"));
}
"##;

    fn input(root: &Path, class: &str, source: &str) -> NativeCompanionInput {
        let path = root.join(format!("client/services/{class}.rs"));
        std::fs::create_dir_all(path.parent().expect("parent")).expect("services");
        std::fs::write(&path, source).expect("service");
        NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Rust,
            source: path,
            service: String::from("root"),
            class: class.to_owned(),
            web_assembly: false,
            application_provider: false,
        }
    }

    fn asking(root: &Path) -> PathBuf {
        Self::asking_from(root, Self::ORIGIN)
    }

    pub(super) fn asking_from(root: &Path, origin: &str) -> PathBuf {
        for (relative, source) in [
            ("client/models/Greeting.rs", Self::GREETING),
            ("client/remotes/GreetingClient.rs", Self::CLIENT),
            ("client/setups/GreetingSetup.rs", Self::SETUP),
        ] {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, source).expect("source");
        }
        let input = Self::input(root, "CounterService", Self::ASKING);
        super::rust::RustCompanion::stage_reaching(&[input], root, ("org.tachyon.reach", origin))
            .expect("staged")
            .expect("native source")
    }

    fn windows_standard_library(target: &str) -> bool {
        let output = std::process::Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output();
        output
            .ok()
            .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
            .is_some_and(|folder| folder.join("libstd.rlib").exists() | Self::has_std(&folder))
    }

    pub(super) async fn serve(listener: tokio::net::TcpListener, heads: Arc<Mutex<Vec<String>>>) {
        while let Ok((stream, _)) = listener.accept().await {
            Self::answer(stream, &heads).await;
        }
    }

    async fn answer(mut stream: tokio::net::TcpStream, heads: &Mutex<Vec<String>>) {
        let mut head = Vec::new();
        let mut chunk = [0u8; 1024];
        while !head.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut chunk).await.expect("request head");
            assert!(read > 0, "the client closed before its request head ended");
            head.extend_from_slice(&chunk[..read]);
        }
        let head = String::from_utf8_lossy(&head).into_owned();
        let greeting = head.starts_with("GET /api/greeting HTTP/1.1\r\n");
        heads.lock().expect("heads").push(head);
        let (status, body) = [
            ("422 Unprocessable Content", r#"{"message":"refused"}"#),
            ("200 OK", r#"{"message":"hello from Yon"}"#),
        ][usize::from(greeting)];
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(reply.as_bytes()).await.expect("response");
        stream.shutdown().await.expect("close");
    }

    async fn windows_executable(source: &Path, caller: &str) -> PathBuf {
        let staged = std::fs::read_to_string(source).expect("staged source");
        std::fs::write(source, staged + caller).expect("native caller");
        let executable = source.with_extension("exe");
        let mut command = tokio::process::Command::new("rustc");
        command
            .args(["--edition", "2024"])
            .args(["--target", WindowsHostTests::RUST_TARGET_FOR_WINE])
            .arg(format!("-Clinker={}", WindowsHostTests::MINGW_FOR_WINE))
            .arg(source)
            .arg("-o")
            .arg(&executable);
        let compiled =
            crate::external_command::run(&mut command, std::time::Duration::from_mins(3), 65536)
                .await
                .expect("Rust compile");
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        WindowsHostTests::stage_wine_runtime(executable.parent().expect("folder"));
        executable
    }

    async fn under_wine(executable: &Path, prefix: &Path) -> crate::external_command::ToolOutput {
        let mut command = tokio::process::Command::new(WindowsHostTests::wine().expect("Wine"));
        command
            .arg(executable)
            .env("WINEDEBUG", "-all")
            .env("WINEPREFIX", prefix)
            .env("WINEDLLOVERRIDES", "mscoree,mshtml=");
        let started = std::time::Instant::now();
        let executed =
            crate::external_command::run(&mut command, std::time::Duration::from_mins(5), 65536)
                .await
                .expect("wine execute");
        eprintln!("wine run took {:?}", started.elapsed());
        let mut stop = tokio::process::Command::new("wineserver");
        stop.arg("-k").env("WINEPREFIX", prefix);
        let _ = crate::external_command::run(&mut stop, std::time::Duration::from_secs(30), 65536)
            .await;
        executed
    }

    async fn reach_under_wine(root: &Path) -> (String, Vec<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let origin = format!("http://{}", listener.local_addr().expect("address"));
        let heads = Arc::new(Mutex::new(Vec::new()));
        let server = tokio::spawn(Self::serve(listener, heads.clone()));
        let source = Self::asking_from(root, &origin);
        let executable = Self::windows_executable(&source, Self::REACHING_CALLER).await;
        let executed = Self::under_wine(&executable, &root.join("wine")).await;
        server.abort();
        let stdout = String::from_utf8_lossy(&executed.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&executed.stderr);
        assert!(executed.status.success(), "{stdout}\n{stderr}");
        let heads = heads.lock().expect("heads").clone();
        (stdout, heads)
    }

    fn has_std(folder: &Path) -> bool {
        std::fs::read_dir(folder).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().starts_with("libstd-"))
        })
    }
}

#[tokio::test]
async fn native_rust_tasks_park_until_the_host_resumes_them() {
    let root = tempfile::tempdir().expect("fixture");
    let input =
        NativeRustTaskCheck::input(root.path(), "WaitService", NativeRustTaskCheck::WAITING);
    let source = super::rust::stage(&[input], root.path(), "org.tachyon.tasks")
        .expect("staged")
        .expect("native source");
    NativePageGraphCheck::execute(&source, NativeRustTaskCheck::WAITING_CALLER).await;
}

#[tokio::test]
async fn native_rust_async_listeners_run_in_order_through_the_host_dispatch() {
    let root = tempfile::tempdir().expect("fixture");
    let models = root.path().join("client/models");
    std::fs::create_dir_all(&models).expect("models");
    std::fs::write(models.join("Ping.rs"), NativeRustTaskCheck::PING).expect("model");
    let input =
        NativeRustTaskCheck::input(root.path(), "ListenService", NativeRustTaskCheck::LISTENING);
    let source = super::rust::stage(&[input], root.path(), "org.tachyon.listeners")
        .expect("staged")
        .expect("native source");
    NativePageGraphCheck::execute(&source, NativeRustTaskCheck::LISTENING_CALLER).await;
}

#[tokio::test]
async fn native_rust_http_clients_carry_the_yon_origin_and_the_windows_transport() {
    let root = tempfile::tempdir().expect("fixture");
    let source = NativeRustTaskCheck::asking(root.path());
    let staged = std::fs::read_to_string(&source).expect("staged source");
    for expected in [
        "const TAC_YON_ORIGIN: &str = \"https://yon.example.com\";",
        "Ok(exchange) => Self::answer(exchange.await),",
        "impl TacWinHttp {",
        "#[link(name = \"winhttp\")]",
    ] {
        assert!(staged.contains(expected), "{expected}");
    }
    assert!(!staged.contains("drop(text);\n        Err(behavior_error("));
    NativePageGraphCheck::execute(&source, NativeRustTaskCheck::ASKING_CALLER).await;
}

#[tokio::test]
async fn native_rust_http_transport_compiles_for_windows() {
    if !NativeRustTaskCheck::windows_standard_library("x86_64-pc-windows-gnu") {
        eprintln!("skipping: the x86_64-pc-windows-gnu standard library is not installed");
        return;
    }
    let root = tempfile::tempdir().expect("fixture");
    let source = NativeRustTaskCheck::asking(root.path());
    let mut command = tokio::process::Command::new("rustc");
    command
        .args([
            "--edition",
            "2024",
            "--crate-type",
            "cdylib",
            "--emit",
            "metadata",
        ])
        .args(["--target", "x86_64-pc-windows-gnu"])
        .arg(&source)
        .arg("--out-dir")
        .arg(root.path());
    let compiled =
        crate::external_command::run(&mut command, std::time::Duration::from_mins(2), 65536)
            .await
            .expect("Rust compile");
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
}

#[tokio::test]
async fn native_rust_http_transport_reaches_a_local_yon_under_wine() {
    let target = WindowsHostTests::RUST_TARGET_FOR_WINE;
    let wine = WindowsHostTests::wine();
    if !NativeRustTaskCheck::windows_standard_library(target) | wine.is_none() {
        eprintln!("skipping: the {target} standard library or Wine is not installed");
        return;
    }
    let root = tempfile::tempdir().expect("fixture");
    let (stdout, heads) = NativeRustTaskCheck::reach_under_wine(root.path()).await;
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 3, "{stdout}");
    assert!(
        lines[0].ends_with(r#""answer":"{\"value\":\"hello from Yon\"}"}"#),
        "{stdout}"
    );
    assert!(
        lines[1].ends_with(r#""answer":"{\"error\":\"The browser HTTP request failed.\",\"code\":\"HTTP_STATUS\"}"}"#),
        "{stdout}"
    );
    assert_eq!(lines[2], r#"{"value":"hello from Yon"}"#, "{stdout}");
    assert_eq!(heads.len(), 2, "{heads:?}");
    assert!(
        heads[0].starts_with("GET /api/greeting HTTP/1.1\r\n"),
        "{heads:?}"
    );
    assert!(
        heads[1].starts_with("GET /api/refused HTTP/1.1\r\n"),
        "{heads:?}"
    );
    assert!(
        heads
            .iter()
            .all(|head| !head.to_ascii_lowercase().contains("\r\ncookie:")),
        "{heads:?}"
    );
}
