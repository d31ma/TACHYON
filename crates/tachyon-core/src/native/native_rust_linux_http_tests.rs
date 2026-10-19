#![allow(clippy::expect_used)]

#[cfg(target_os = "linux")]
use super::native_page_graph_tests::NativePageGraphCheck;
#[cfg(target_os = "linux")]
use super::registry::NativeCompanionInput;
#[cfg(target_os = "linux")]
use crate::project::NativeCompanion;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
struct LinuxHttpCheck;

#[cfg(target_os = "linux")]
impl LinuxHttpCheck {
    const ORIGIN: &'static str = "https://yon.example.com";
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/models/Greeting.rs",
            "pub struct Greeting {\n    pub message: String,\n}\n",
        ),
        (
            "client/remotes/GreetingClient.rs",
            "use crate::models::Greeting;\n\n#[HTTPClient(\"/api\")]\ntrait GreetingClient {\n    #[GetRequest(\"/greeting\")]\n    async fn get_greeting(&self) -> BehaviorResult<Greeting>;\n    #[PostRequest(\"/refused\")]\n    async fn post_refused(&self, #[RequestHeader(\"Authorization\")] token: String) -> BehaviorResult<Greeting>;\n    #[GetRequest(\"/large\")]\n    async fn get_large(&self) -> BehaviorResult<Greeting>;\n}\n",
        ),
        (
            "client/setups/GreetingSetup.rs",
            "#[Setup]\nstruct GreetingSetup {}\nimpl GreetingSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n",
        ),
        (
            "client/services/AskingService.rs",
            "#[Service]\npub struct AskingService {\n    #[Needs]\n    greeting_client: std::sync::Arc<GreetingClient>,\n    pub message: String,\n}\n\nimpl AskingService {\n    pub async fn ask_server(&mut self) -> BehaviorResult<String> {\n        self.message = self.greeting_client.get_greeting().await?.message;\n        Ok(self.message.clone())\n    }\n\n    pub async fn ask_refused(&mut self) -> BehaviorResult<String> {\n        Ok(self.greeting_client.post_refused(String::from(\"member-token\")).await?.message)\n    }\n\n    pub async fn ask_large(&mut self) -> BehaviorResult<String> {\n        Ok(self.greeting_client.get_large().await?.message)\n    }\n}\n",
        ),
    ];
    const CALLER: &'static str = r##"
struct FakeGtkHost;

impl FakeGtkHost {
    const OWNER: &'static str = "00000000-0000-4000-8000-000000000091";

    fn signals() -> &'static std::sync::Mutex<Vec<String>> {
        static SIGNALS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        &SIGNALS
    }

    fn sent() -> &'static std::sync::Mutex<Vec<String>> {
        static SENT: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        &SENT
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

    fn text(pointer: *const core::ffi::c_char) -> String {
        unsafe { std::ffi::CStr::from_ptr(pointer) }.to_string_lossy().into_owned()
    }

    extern "C" fn send(
        ticket: u64,
        method: *const core::ffi::c_char,
        url: *const core::ffi::c_char,
        headers: *const core::ffi::c_char,
        body: *const u8,
        length: usize,
        timeout: u64,
        limit: usize,
    ) {
        let url = Self::text(url);
        let line = format!("{} {url} {:?} body={} {} {limit}", Self::text(method), Self::text(headers), !body.is_null() && length > 0, timeout > 0);
        Self::sent().lock().expect("sent").push(line);
        std::thread::spawn(move || {
            let (status, answer, error): (u32, &[u8], &[u8]) = match url.rsplit('/').next() {
                Some("greeting") => (200, br#"{"message":"hello from Yon"}"#, b"\0"),
                Some("refused") => (401, b"", b"\0"),
                _ => (0, b"", b"HTTP_LIMIT\0"),
            };
            let error = if error.len() > 1 { error.as_ptr().cast() } else { core::ptr::null() };
            unsafe { TacLinuxHttp::tac_native_http_answer(ticket, status, answer.as_ptr(), answer.len(), error) };
        });
    }

    fn request(operation: &str, name: &str) -> String {
        tac_handle(&format!(
            r#"{{"route":"/","service":"asking","owner":"{}","op":"{operation}","name":"{name}","args":[]}}"#,
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
    let init = FakeGtkHost::request("init", "");
    assert!(!init.contains("error"), "{init}");
    unsafe { tac_native_set_emit(Some(FakeGtkHost::emit)) };
    unsafe { TacTaskDispatch::tac_native_set_dispatch(Some(FakeGtkHost::dispatch)) };
    let missing = FakeGtkHost::ask("askServer");
    assert!(missing.contains("HTTP_MISSING"), "a host without an HTTP sender keeps refusing: {missing}");
    unsafe { TacLinuxHttp::tac_native_set_http_sender(Some(FakeGtkHost::send)) };
    let answered = FakeGtkHost::ask("askServer");
    assert!(answered.ends_with(r#""answer":"{\"value\":\"hello from Yon\"}"}"#), "{answered}");
    let refused = FakeGtkHost::ask("askRefused");
    assert!(refused.contains("HTTP_STATUS"), "{refused}");
    let large = FakeGtkHost::ask("askLarge");
    assert!(large.contains("HTTP_LIMIT"), "{large}");
    let message = FakeGtkHost::request("get", "message");
    assert_eq!(message, r#"{"value":"hello from Yon"}"#);
    let sent = FakeGtkHost::sent().lock().expect("sent").clone();
    assert_eq!(sent.len(), 3, "{sent:?}");
    assert!(sent[0].starts_with("GET https://yon.example.com/api/greeting \"\" body=false true "), "{sent:?}");
    assert!(sent[1].starts_with("POST https://yon.example.com/api/refused \"authorization\\nmember-token"), "{sent:?}");
    assert!(sent[2].starts_with("GET https://yon.example.com/api/large "), "{sent:?}");
    println!("linked Rust HTTP clients reach Yon through the GTK host's sender");
}
"##;

    fn stage(root: &Path) -> PathBuf {
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, source).expect("source");
        }
        let input = NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Rust,
            source: root.join("client/services/AskingService.rs"),
            service: String::from("asking"),
            class: String::from("AskingService"),
            web_assembly: false,
            application_provider: false,
        };
        super::rust::RustCompanion::stage_reaching(
            &[input],
            root,
            ("org.tachyon.linux-http", Self::ORIGIN),
        )
        .expect("staged")
        .expect("native source")
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn linked_rust_http_clients_reach_yon_through_the_gtk_host_sender() {
    let root = tempfile::tempdir().expect("fixture");
    let source = LinuxHttpCheck::stage(root.path());
    NativePageGraphCheck::execute(&source, LinuxHttpCheck::CALLER).await;
}

#[test]
fn the_linux_host_resumes_tasks_on_its_main_loop_and_sends_yon_calls_through_libsoup() {
    let source = super::linux::host_source();
    assert!(source.contains("__NATIVE_TASKS__\n\nstatic void tachyon_companion_load(void) {"));
    assert!(source.contains("    tachyon_tasks_bind(module);\n"));
    let tasks = include_str!("linux_task_http.c");
    for expected in [
        "g_module_symbol(module, \"tac_native_set_dispatch\", (gpointer *)&set_dispatch);",
        "if (g_atomic_int_compare_and_exchange(&g_resume_posted, 0, 1)) g_idle_add(tachyon_task_resume, NULL);",
        "g_companion_invoke(\"{\\\"op\\\":\\\"host-resume\\\"}\");",
        "g_module_symbol(module, \"tac_native_http_answer\", (gpointer *)&g_http_answer);",
        "soup_message_add_flags(call->message, SOUP_MESSAGE_NO_REDIRECT);",
        "if (call->received->len + size > call->limit) { g_bytes_unref(chunk); tachyon_http_finish(call, 0, \"HTTP_LIMIT\"); return; }",
        "return call->expired ? \"DEADLINE\" : \"HTTP_NETWORK\";",
        "g_idle_add(tachyon_http_begin, call);",
    ] {
        assert!(tasks.contains(expected), "{expected}");
    }
}
