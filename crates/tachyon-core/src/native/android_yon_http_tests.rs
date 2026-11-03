#![allow(clippy::expect_used)]

use super::managed_capability_tests::ManagedCapabilityCheck;
use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

struct KotlinYonHttpCheck;

impl KotlinYonHttpCheck {
    const MODEL: &'static str = "data class Greeting(val message: String)\n";
    const CLIENT: &'static str = "import models.Greeting\n\n@HTTPClient(\"/api\")\ninterface GreetingClient {\n    @GetRequest(\"/greeting\")\n    suspend fun read(@RequestParam(\"name\") name: String, @RequestHeader(\"authorization\") token: String): Greeting\n\n    @PostRequest(\"/greeting\")\n    suspend fun save(@RequestBody name: String): Greeting\n\n    @GetRequest(\"/missing\")\n    suspend fun missing(): Greeting\n\n    @GetRequest(\"/moved\")\n    suspend fun moved(): Greeting\n\n    @GetRequest(\"/huge\")\n    suspend fun huge(): Greeting\n}\n";
    const SETUP: &'static str = "@Setup\nclass GreetingSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n";
    const SERVICE: &'static str = "@Service\nclass ReachService {\n    @Needs\n    private val greetingClient: GreetingClient\n    var message: String = \"\"\n\n    suspend fun ask() { message = greetingClient.read(\"Ada Lovelace\", \"Bearer token-1\").message }\n\n    suspend fun save() { message = greetingClient.save(\"from Kotlin\").message }\n\n    suspend fun missing() { message = failure { greetingClient.missing() } }\n\n    suspend fun moved() { message = failure { greetingClient.moved() } }\n\n    suspend fun huge() { message = failure { greetingClient.huge() } }\n\n    private suspend fun failure(call: suspend () -> models.Greeting): String =\n        try { call().message } catch (error: Behaviors.Failure) { error.code }\n}\n";
    const CALLER: &'static str = r#"
fun ask(op: String, extra: String = ""): String =
    tacInvokeOwned("{\"route\":\"/\",\"service\":\"reach\",\"owner\":\"00000000-0000-4000-8000-000000000001\",\"op\":\"$op\"$extra}")

fun main() {
    check(!ask("init").contains("error")) { "init failed" }
    for (name in listOf("ask", "save", "missing", "moved", "huge")) {
        val answer = ask("call", ",\"name\":\"$name\",\"args\":[]")
        val message = ask("get", ",\"name\":\"message\"")
        println("$name $answer $message")
    }
}
"#;
    const HUGE_BYTES: usize = 1_048_600;

    fn registry(root: &std::path::Path) -> String {
        for (relative, source) in [
            ("client/models/Greeting.kt", Self::MODEL),
            ("client/remotes/GreetingClient.kt", Self::CLIENT),
            ("client/setups/GreetingSetup.kt", Self::SETUP),
            ("client/services/ReachService.kt", Self::SERVICE),
        ] {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, source).expect("source");
        }
        let input = NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Kotlin,
            source: root.join("client/services/ReachService.kt"),
            service: String::from("reach"),
            class: String::from("ReachService"),
            web_assembly: false,
            application_provider: false,
        };
        super::registry::source(&[input], NativeCompanion::Kotlin)
            .expect("registry")
            .expect("source")
    }

    fn run(origin: &str) -> String {
        let root = tempfile::tempdir().expect("project");
        let linked = super::android_yon_reach::AndroidYonReach::declared(Some(origin))
            .linked(&Self::registry(root.path()));
        let models = format!("package models\n\n{}", Self::MODEL);
        std::fs::create_dir_all(root.path().join("jvm")).expect("sources");
        std::fs::write(root.path().join("jvm/Models.kt"), models).expect("models");
        ManagedCapabilityCheck::kotlin_jvm(root.path(), &linked, Self::CALLER, true)
    }

    fn serve(listener: &TcpListener, seen: &std::sync::mpsc::Sender<String>) {
        for stream in listener.incoming().take(5) {
            Self::answer(stream.expect("connection"), seen);
        }
    }

    fn answer(mut stream: TcpStream, seen: &std::sync::mpsc::Sender<String>) {
        let mut reader = BufReader::new(stream.try_clone().expect("stream"));
        let mut head = String::new();
        let mut line = String::new();
        while reader.read_line(&mut line).expect("line") > 2 {
            head.push_str(&line);
            line.clear();
        }
        let length = head
            .lines()
            .find_map(|header| {
                header
                    .to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_owned)
            })
            .map_or(0, |value| value.trim().parse::<usize>().expect("length"));
        let mut body = vec![0; length];
        reader.read_exact(&mut body).expect("body");
        let request = format!("{head}\n{}", String::from_utf8_lossy(&body));
        seen.send(request.clone()).expect("seen");
        let (status, text) = Self::route(&request);
        let reply = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nlocation: /api/greeting\r\nconnection: close\r\n\r\n{text}",
            text.len()
        );
        stream.write_all(reply.as_bytes()).expect("reply");
    }

    fn route(request: &str) -> (&'static str, String) {
        let first = request.lines().next().unwrap_or_default();
        match first.split(' ').take(2).collect::<Vec<_>>().as_slice() {
            ["GET", "/api/greeting?name=Ada+Lovelace"] => {
                ("200 OK", String::from("{\"message\":\"Hello Ada\"}"))
            }
            ["POST", "/api/greeting"] => (
                "200 OK",
                String::from("{\"message\":\"Saved from Kotlin\"}"),
            ),
            ["GET", "/api/moved"] => ("302 Found", String::new()),
            ["GET", "/api/huge"] => (
                "200 OK",
                format!("{{\"message\":\"{}\"}}", "a".repeat(Self::HUGE_BYTES)),
            ),
            _ => ("404 Not Found", String::from("{}")),
        }
    }
}

#[test]
fn a_linked_kotlin_http_client_reaches_yon_through_the_native_transport() {
    if !ManagedCapabilityCheck::available("kotlinc", "-version")
        || !ManagedCapabilityCheck::available("java", "-version")
    {
        eprintln!("skipping: Kotlin JVM tools are unavailable");
        return;
    }
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let origin = format!(
        "http://127.0.0.1:{}",
        listener.local_addr().expect("address").port()
    );
    let (seen, requests) = std::sync::mpsc::channel();
    std::thread::spawn(move || KotlinYonHttpCheck::serve(&listener, &seen));
    let output = KotlinYonHttpCheck::run(&origin);
    let requests: Vec<String> = requests.try_iter().collect();
    println!("{output}");
    let line = |name: &str| {
        output
            .lines()
            .find(|line| line.starts_with(&format!("{name} ")))
            .unwrap_or_default()
            .to_owned()
    };
    assert!(
        line("ask").ends_with("{\"value\":\"Hello Ada\"}"),
        "{output}"
    );
    assert!(
        line("save").ends_with("{\"value\":\"Saved from Kotlin\"}"),
        "{output}"
    );
    for (name, code) in [
        ("missing", "HTTP_STATUS"),
        ("moved", "HTTP_NETWORK"),
        ("huge", "HTTP_LIMIT"),
    ] {
        assert!(
            line(name).ends_with(&format!("{{\"value\":\"{code}\"}}")),
            "{name}: {output}"
        );
    }
    let lowered: Vec<String> = requests
        .iter()
        .map(|request| request.to_ascii_lowercase())
        .collect();
    assert!(
        lowered[0].contains("authorization: bearer token-1"),
        "{}",
        requests[0]
    );
    assert!(
        lowered[1].contains("content-type: application/json"),
        "{}",
        requests[1]
    );
    assert!(requests[1].ends_with("\"from Kotlin\""), "{}", requests[1]);
    assert!(
        lowered
            .iter()
            .all(|request| !request.contains("\ncookie:") && !request.contains("\norigin:"))
    );
}

#[test]
fn a_linked_kotlin_http_client_without_a_yon_origin_says_how_to_declare_one() {
    let root = tempfile::tempdir().expect("project");
    let registry = KotlinYonHttpCheck::registry(root.path());
    assert!(registry.contains("The default HTTP transport runs only in the browser."));
    let linked = super::android_yon_reach::AndroidYonReach::declared(None).linked(&registry);
    assert!(!linked.contains("The default HTTP transport runs only in the browser."));
    assert!(linked.contains("private const val ORIGIN = \"\""));
    assert!(linked.contains("Set tac.yon.origin in client/manifest.json"));
    let declared =
        super::android_yon_reach::AndroidYonReach::declared(Some("https://api.example.com"))
            .linked(&registry);
    assert!(declared.contains("private const val ORIGIN = \"https://api.example.com\""));
}
