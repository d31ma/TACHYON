#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

struct Language {
    extension: &'static str,
    program: &'static str,
}

impl Language {
    const JAVASCRIPT: Self = Self::new("js", "node");
    const TYPESCRIPT: Self = Self::new("ts", "node");
    const PYTHON: Self = Self::new("py", "python3");
    const PHP: Self = Self::new("php", "php-cgi");
    const JAVA: Self = Self::new("java", "javac");
    const GROOVY: Self = Self::new("groovy", "groovyc");
    const KOTLIN: Self = Self::new("kt", "kotlinc");
    const CSHARP: Self = Self::new("cs", "dotnet");
    const FSHARP: Self = Self::new("fs", "dotnet");
    const RUST: Self = Self::new("rs", "rustc");
    const DART: Self = Self::new("dart", "dart");
    const GO: Self = Self::new("go", "go");
    const RUBY: Self = Self::new("rb", "ruby");
    const LUA: Self = Self::new("lua", "lua");
    const CPP: Self = Self::new("hpp", "clang++");

    const fn new(extension: &'static str, program: &'static str) -> Self {
        Self { extension, program }
    }

    fn available(&self) -> bool {
        let program = if self.program == "node" {
            std::env::var("YON_JAVASCRIPT_RUNTIME").unwrap_or_else(|_| String::from("node"))
        } else {
            self.program.to_owned()
        };
        let flag = match program.as_str() {
            "go" => "version",
            "kotlinc" => "-version",
            "lua" => "-v",
            _ => "--version",
        };
        let found = crate::support::program_probe::ProgramProbe::answers(&program, flag);
        assert!(
            found || std::env::var_os("CI").is_none(),
            "{program} is missing in CI"
        );
        if !found {
            eprintln!("skipping .{}: {program} is unavailable", self.extension);
        }
        found
    }

    async fn run_access_section(&self) {
        if !self.available() {
            return;
        }
        let (_access, server) = Section::ACCESS.serve(self).await;
        Section::check_access(&server, self).await;
        server.stop().await;
    }

    async fn run_transaction_section(&self) {
        if !self.available() {
            return;
        }
        let (_ledger, server) = Section::TRANSACTIONS.serve(self).await;
        Section::check_transaction(&server, self).await;
        server.stop().await;
    }

    async fn run_named_section(&self) {
        if !self.available() {
            return;
        }
        let (_products, server) = Section::NAMED.serve(self).await;
        Section::check_named(&server, self).await;
        server.stop().await;
        Section::NAMED.refuse_an_unknown_name(self);
    }

    async fn run_delegate_section(&self) {
        if !self.available() {
            return;
        }
        let (_taxes, server) = Section::DELEGATES.serve(self).await;
        Section::check_delegate(&server, self).await;
        server.stop().await;
        Section::DELEGATES.refuse_an_overload(self);
    }
}

struct Section {
    folder: &'static str,
}

impl Section {
    const ACCESS: Self = Self {
        folder: "docs/yon/who-may-call-a-route-in-every-yon-language",
    };
    const TRANSACTIONS: Self = Self {
        folder: "docs/annotations/transactions-yon",
    };
    const NAMED: Self = Self {
        folder: "docs/yon/annotated-controllers-and-providers",
    };
    const DELEGATES: Self = Self {
        folder: "docs/delegates/take-on-a-service",
    };
    const ONE_METHOD_PER_NAME: [&'static str; 3] = ["go", "rs", "dart"];
    const SAME_NAME: [(&'static str, &'static str); 3] = [
        ("TotalWith", "Total"),
        ("total_with", "total"),
        ("totalWith", "total"),
    ];
    const NAMED_PROVIDER: [(&'static str, &'static str); 2] = [
        ("(\"ProductsService\")", "(\"MissingService\")"),
        ("('ProductsService')", "('MissingService')"),
    ];

    fn source(&self) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../website/content")
            .join(self.folder)
    }

    fn files(&self, language: &Language) -> Vec<(String, String)> {
        let mut found = Vec::new();
        Self::collect(&self.source(), &self.source(), &mut found);
        let suffix = format!(".{}", language.extension);
        let own: Vec<(String, String)> = found
            .iter()
            .filter(|(relative, _)| relative.ends_with(&suffix))
            .cloned()
            .collect();
        let own_code: String = own.iter().map(|(_, code)| code.as_str()).collect();
        let shared = found.into_iter().filter(|(relative, _)| {
            let realm_path = relative.trim_start_matches("server");
            !relative.ends_with(&suffix) && own_code.contains(realm_path)
        });
        own.into_iter().chain(shared).collect()
    }

    fn collect(base: &Path, folder: &Path, found: &mut Vec<(String, String)>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(folder)
            .expect("published section folder")
            .map(|entry| entry.expect("folder entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                Self::collect(base, &path, found);
                continue;
            }
            let relative = path
                .strip_prefix(base)
                .expect("inside the section")
                .to_string_lossy()
                .replace('\\', "/");
            if relative.starts_with("server/") {
                found.push((relative, fs::read_to_string(&path).expect("source")));
            }
        }
    }

    fn project(&self, language: &Language, renames: &[(&str, &str)]) -> tempfile::TempDir {
        let project = tempfile::tempdir().expect("project");
        Project::write(project.path(), "client/pages/tac.html", "<test-view1 />");
        Project::write(
            project.path(),
            "client/components/test/view1/tac.html",
            "<main aria-label=\"Home\"><h1>Home</h1></main>",
        );
        let files = self.files(language);
        assert!(
            !files.is_empty(),
            "{} has no .{} files",
            self.folder,
            language.extension
        );
        for (relative, code) in files {
            let written = renames
                .iter()
                .fold(code, |text, (from, to)| text.replace(from, to));
            Project::write(project.path(), &relative, &written);
        }
        project
    }

    async fn serve(&self, language: &Language) -> (tempfile::TempDir, Server) {
        let project = self.project(language, &[]);
        let server = Server::start(project.path()).await;
        (project, server)
    }

    async fn check_access(server: &Server, language: &Language) {
        let label = language.extension;
        let public = server.call("GET", "/orders", "").await;
        assert_eq!(public.status, 200, "{label}: {public:?}");
        assert!(public.body.contains("lamp"), "{label}: {public:?}");
        let anonymous = server.call("POST", "/orders", "").await;
        assert_eq!(anonymous.status, 401, "{label}: {anonymous:?}");
        let bob = "authorization: bob-token\r\n";
        let signed_in = server.call("POST", "/orders", bob).await;
        assert_eq!(signed_in.status, 200, "{label}: {signed_in:?}");
        assert!(signed_in.body.contains("chair"), "{label}: {signed_in:?}");
        let anonymous_delete = server.call("DELETE", "/orders", "").await;
        assert_eq!(
            anonymous_delete.status, 401,
            "{label}: {anonymous_delete:?}"
        );
        let refused = server.call("DELETE", "/orders", bob).await;
        assert_eq!(refused.status, 403, "{label}: {refused:?}");
        let ada = "authorization: ada-token\r\n";
        let allowed = server.call("DELETE", "/orders", ada).await;
        assert_eq!(allowed.status, 200, "{label}: {allowed:?}");
    }

    async fn check_transaction(server: &Server, language: &Language) {
        let label = language.extension;
        let answer = server.call("GET", "/ledger", "").await;
        assert_eq!(answer.status, 200, "{label}: {answer:?}");
        let report: serde_json::Value = serde_json::from_str(&answer.body).expect("JSON report");
        let commands: Vec<&str> = report["commands"]
            .as_array()
            .expect("command list")
            .iter()
            .map(|command| command.as_str().expect("command text"))
            .collect();
        let store_logged = commands.iter().any(|command| command.starts_with("find@"));
        let expected: &[&str] = if store_logged {
            &["begin", "find@t1", "count@t1", "commit:t1"]
        } else {
            &["begin", "commit:t1"]
        };
        assert_eq!(
            &commands[..expected.len().min(commands.len())],
            expected,
            "{label}: {report}"
        );
        if !store_logged {
            assert_eq!(
                report["settled"]["open"][0]["transaction"], "t1",
                "{label}: {report}"
            );
        }
    }

    async fn check_named(server: &Server, language: &Language) {
        let label = language.extension;
        let list = server.call("GET", "/products", "").await;
        assert_eq!(list.status, 200, "{label}: {list:?}");
        let listed: serde_json::Value = serde_json::from_str(&list.body).expect("JSON list");
        assert_eq!(
            listed,
            serde_json::json!([{"id": 1, "name": "Anvil"}]),
            "{label}"
        );
        let one = server.call("GET", "/products/7", "").await;
        assert_eq!(one.status, 200, "{label}: {one:?}");
        let found: serde_json::Value = serde_json::from_str(&one.body).expect("JSON product");
        assert_eq!(
            found,
            serde_json::json!({"id": 7, "name": "Anvil"}),
            "{label}"
        );
    }

    async fn check_delegate(server: &Server, language: &Language) {
        let label = language.extension;
        let answer = server.call("GET", "/tax", "").await;
        assert_eq!(answer.status, 200, "{label}: {answer:?}");
        let taxes: serde_json::Value = serde_json::from_str(&answer.body).expect("JSON taxes");
        assert_eq!(
            taxes,
            serde_json::json!({"rate": 4, "total": 30, "extra": 34}),
            "{label}: the Delegate overrides rate, takes on total and overloads it"
        );
    }

    fn refuse_an_overload(&self, language: &Language) {
        if !Self::ONE_METHOD_PER_NAME.contains(&language.extension) {
            return;
        }
        let project = self.project(language, &Self::SAME_NAME);
        let refusal = ProjectDiscovery::discover(project.path())
            .err()
            .map(|failure| failure.to_string())
            .unwrap_or_default();
        assert!(
            refusal.contains("TY1374"),
            "{}: a Delegate method with the name of a base method and other parameters must fail the build: {refusal}",
            language.extension
        );
    }

    fn refuse_an_unknown_name(&self, language: &Language) {
        let project = self.project(language, &Self::NAMED_PROVIDER);
        let refusal = ProjectDiscovery::discover(project.path())
            .err()
            .map(|failure| failure.to_string())
            .unwrap_or_default();
        assert!(
            refusal.contains("MissingService"),
            "{}: a @Named that names no provider must fail the build: {refusal}",
            language.extension
        );
    }
}

struct Project;

impl Project {
    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("source");
    }
}

#[derive(Debug)]
struct Answer {
    status: u16,
    body: String,
}

struct Server {
    port: u16,
    stop: tokio::sync::oneshot::Sender<()>,
    running: tokio::task::JoinHandle<()>,
}

impl Server {
    async fn start(root: &Path) -> Self {
        let server = DevServer::bind(
            root,
            &DevServerOptions {
                port: 0,
                watch: false,
                ..DevServerOptions::default()
            },
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
        let port = server.address().port();
        let (stop, shutdown) = tokio::sync::oneshot::channel();
        let running = tokio::spawn(async move {
            let _served = server
                .run_until(async {
                    let _stopped = shutdown.await;
                })
                .await;
        });
        Self {
            port,
            stop,
            running,
        }
    }

    async fn call(&self, method: &str, path: &str, headers: &str) -> Answer {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))
            .await
            .expect("connect");
        let raw = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{headers}Content-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(raw.as_bytes()).await.expect("write");
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_mins(5), stream.read_to_end(&mut response))
            .await
            .expect("answer within the deadline")
            .expect("read");
        let text = String::from_utf8_lossy(&response).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or_default();
        Answer {
            status,
            body: Self::unchunked(head, body),
        }
    }

    fn unchunked(head: &str, body: &str) -> String {
        if !head
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked")
        {
            return body.to_owned();
        }
        let mut text = String::new();
        let mut rest = body;
        while let Some((size, after)) = rest.split_once("\r\n") {
            let length = usize::from_str_radix(size.trim(), 16).unwrap_or_default();
            if length == 0 {
                break;
            }
            text.push_str(&after[..length]);
            rest = after[length..].trim_start_matches("\r\n");
        }
        text
    }

    async fn stop(self) {
        let _stopped = self.stop.send(());
        let _finished = self.running.await;
    }
}

macro_rules! published_section_tests {
    ($($module:ident => $language:ident),+ $(,)?) => {
        $(
            mod $module {
                use super::Language;

                #[tokio::test]
                async fn access() {
                    Language::$language.run_access_section().await;
                }

                #[tokio::test]
                async fn transactions() {
                    Language::$language.run_transaction_section().await;
                }

                #[tokio::test]
                async fn named() {
                    Language::$language.run_named_section().await;
                }

                #[tokio::test]
                async fn delegates() {
                    Language::$language.run_delegate_section().await;
                }
            }
        )+
    };
}

published_section_tests!(
    javascript_runs_its_published_yon_sections => JAVASCRIPT,
    typescript_runs_its_published_yon_sections => TYPESCRIPT,
    python_runs_its_published_yon_sections => PYTHON,
    php_runs_its_published_yon_sections => PHP,
    java_runs_its_published_yon_sections => JAVA,
    groovy_runs_its_published_yon_sections => GROOVY,
    kotlin_runs_its_published_yon_sections => KOTLIN,
    csharp_runs_its_published_yon_sections => CSHARP,
    fsharp_runs_its_published_yon_sections => FSHARP,
    rust_runs_its_published_yon_sections => RUST,
    dart_runs_its_published_yon_sections => DART,
    go_runs_its_published_yon_sections => GO,
    ruby_runs_its_published_yon_sections => RUBY,
    lua_runs_its_published_yon_sections => LUA,
    cpp_runs_its_published_yon_sections => CPP,
);
