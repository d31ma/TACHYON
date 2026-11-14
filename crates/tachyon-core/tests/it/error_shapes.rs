#![allow(clippy::expect_used)]

use crate::support::json_http::JsonHttp;
use crate::support::one_test_per_case::one_test_per_case;
use crate::support::program_probe::ProgramProbe;
use std::path::{Path, PathBuf};

struct ShapeLanguage {
    extension: &'static str,
    programs: &'static [&'static str],
}

impl ShapeLanguage {
    const ALL: [Self; 15] = [
        Self::new("java", &["java", "javac"]),
        Self::new("groovy", &["java", "groovyc"]),
        Self::new("hpp", &["clang++"]),
        Self::new("cs", &["dotnet"]),
        Self::new("fs", &["dotnet"]),
        Self::new("js", &["node"]),
        Self::new("ts", &["node"]),
        Self::new("kt", &["java", "kotlinc"]),
        Self::new("dart", &["dart"]),
        Self::new("php", &["php-cgi"]),
        Self::new("rb", &["ruby"]),
        Self::new("py", &["python3"]),
        Self::new("go", &["go"]),
        Self::new("rs", &["cargo", "rustc"]),
        Self::new("lua", &["lua"]),
    ];

    const ROLES: [&'static str; 5] = ["controllers", "models", "roles", "services", "errors"];
    const SHARED_MODELS: [&'static str; 1] = ["Problem.ts"];

    const fn new(extension: &'static str, programs: &'static [&'static str]) -> Self {
        Self {
            extension,
            programs,
        }
    }

    fn named(extension: &str) -> &'static Self {
        Self::ALL
            .iter()
            .find(|language| language.extension == extension)
            .unwrap_or_else(|| panic!("no shape language uses .{extension}"))
    }

    fn available(&self) -> bool {
        let found = self.programs.iter().all(|program| Self::runs(program));
        if !found {
            assert!(
                std::env::var_os("CI").is_none(),
                "{} toolchain is required in CI",
                self.extension
            );
            eprintln!(
                "skipping {} error shapes: missing one of {:?}",
                self.extension, self.programs
            );
        }
        found
    }

    fn runs(program: &str) -> bool {
        let flag = match program {
            "java" | "javac" | "kotlinc" => "-version",
            "lua" => "-v",
            "go" => "version",
            _ => "--version",
        };
        ProgramProbe::answers(program, flag)
    }

    fn write_project(&self, root: &Path) {
        for role in Self::ROLES {
            let source = ErrorShapes::examples().join(role);
            if !source.is_dir() {
                continue;
            }
            let target = root.join("server").join(role);
            std::fs::create_dir_all(&target).expect("role folder");
            for entry in std::fs::read_dir(&source).expect("examples") {
                let path = entry.expect("example").path();
                if self.takes(&path) {
                    let name = path.file_name().expect("file name");
                    std::fs::copy(&path, target.join(name)).expect("copy example");
                }
            }
        }
        std::fs::write(
            root.join("server/application.json"),
            ErrorShapes::APPLICATION,
        )
        .expect("application settings");
    }

    fn takes(&self, path: &Path) -> bool {
        let same_language = path
            .extension()
            .is_some_and(|found| found == self.extension);
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let shared = self.extension == "js" && Self::SHARED_MODELS.contains(&name);
        same_language || shared
    }

    async fn answers(&self) -> Vec<String> {
        let root = tempfile::tempdir().expect("project");
        self.write_project(root.path());
        let requests: Vec<String> = ErrorShapes::cases()
            .iter()
            .map(ShapeCase::request)
            .collect();
        JsonHttp::exchange_all(root.path(), &requests).await
    }
}

struct ShapeCase {
    method: &'static str,
    token: Option<&'static str>,
    body: String,
    status_line: &'static str,
    code: &'static str,
    message: String,
}

impl ShapeCase {
    fn request(&self) -> String {
        let authorization = self
            .token
            .map(|token| format!("Authorization: {token}\r\n"))
            .unwrap_or_default();
        let length = self.body.len();
        format!(
            "{} /orders HTTP/1.1\r\nHost: localhost\r\n{authorization}Content-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{}",
            self.method, self.body
        )
    }

    fn check(&self, extension: &str, response: &str) {
        let (head, body) = response
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("{extension}: {response}"));
        let status_line = head.lines().next().unwrap_or_default();
        assert_eq!(status_line, self.status_line, "{extension}: {response}");
        if self.code.is_empty() {
            return;
        }
        let answered: serde_json::Value =
            serde_json::from_str(body).unwrap_or_else(|_| panic!("{extension}: {response}"));
        assert_eq!(answered["code"], self.code, "{extension}: {response}");
        assert_eq!(
            answered["message"],
            self.message.as_str(),
            "{extension}: {response}"
        );
        let lower = head.to_ascii_lowercase();
        assert!(
            lower.contains("x-tachyon-request-id: "),
            "{extension}: {head}"
        );
        assert!(
            lower.contains("cache-control: no-store"),
            "{extension}: {head}"
        );
    }
}

struct ErrorShapes;

impl ErrorShapes {
    const APPLICATION: &'static str = r#"{"server":{"maxbody":1024}}"#;

    fn examples() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../website/content/docs/yon/error-shapes-in-every-yon-language/server")
    }

    fn case(
        method: &'static str,
        token: Option<&'static str>,
        body: &str,
        status_line: &'static str,
        code: &'static str,
        message: &str,
    ) -> ShapeCase {
        ShapeCase {
            method,
            token,
            body: String::from(body),
            status_line,
            code,
            message: String::from(message),
        }
    }

    fn cases() -> Vec<ShapeCase> {
        let large = format!(r#"{{"name":"{}","level":3}}"#, "a".repeat(2048));
        vec![
            Self::case(
                "GET",
                None,
                "",
                "HTTP/1.1 401 Unauthorized",
                "not_signed_in",
                "[MS0001]: Sign in again.",
            ),
            Self::case(
                "GET",
                Some("bob-token"),
                "",
                "HTTP/1.1 403 Forbidden",
                "forbidden",
                "[MS0002]: You may not do this.",
            ),
            Self::case(
                "GET",
                Some("broken-token"),
                "",
                "HTTP/1.1 500 Internal Server Error",
                "identity_failed",
                "[MS0003]: Try again later.",
            ),
            Self::case(
                "POST",
                Some("ada-token"),
                "{",
                "HTTP/1.1 400 Bad Request",
                "invalid_request",
                "[MS0004]: body.",
            ),
            Self::case(
                "POST",
                Some("ada-token"),
                &large,
                "HTTP/1.1 413 Payload Too Large",
                "too_large",
                "[MS0005]: 1024",
            ),
            Self::case(
                "POST",
                Some("ada-token"),
                r#"{"name":"","level":0}"#,
                "HTTP/1.1 422 Unprocessable Entity",
                "invalid_input",
                "[MS0006]: level:Min,name:NotEmpty,name:Length",
            ),
            Self::case(
                "GET",
                Some("ada-token"),
                "",
                "HTTP/1.1 500 Internal Server Error",
                "unexpected",
                "[MS0000]: Something went wrong.",
            ),
            Self::case(
                "POST",
                Some("ada-token"),
                r#"{"name":"lamp","level":3}"#,
                "HTTP/1.1 200 OK",
                "",
                "",
            ),
        ]
    }
}

async fn shapes_every_built_in_answer_in(extension: &str) {
    let language = ShapeLanguage::named(extension);
    if !language.available() {
        return;
    }
    let answers = language.answers().await;
    for (case, response) in ErrorShapes::cases().iter().zip(&answers) {
        case.check(extension, response);
    }
}

one_test_per_case!(
    an_error_handler_shapes_every_answer_tachyon_makes_itself,
    async shapes_every_built_in_answer_in,
    all ShapeLanguage::ALL.map(|language| language.extension),
    [
        java => "java",
        groovy => "groovy",
        cpp => "hpp",
        csharp => "cs",
        fsharp => "fs",
        javascript => "js",
        typescript => "ts",
        kotlin => "kt",
        dart => "dart",
        php => "php",
        ruby => "rb",
        python => "py",
        go => "go",
        rust => "rs",
        lua => "lua",
    ]
);

struct RootHandler {
    extension: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl RootHandler {
    const ALL: [Self; 3] = [
        Self {
            extension: "js",
            files: &[(
                "server/controllers/OrdersErrorHandler.js",
                "@ErrorHandler\nexport class OrdersErrorHandler {\n  /**\n   * @param {Error} error\n   * @returns {{ code: string }}\n   */\n  @HandlesError\n  @ResponseStatus(409)\n  caught(error) {\n    return { code: 'caught' }\n  }\n}\n",
            )],
        },
        Self {
            extension: "go",
            files: &[
                (
                    "server/models/Caught.go",
                    "type Caught struct {\n\tCode string `json:\"code\"`\n}\n",
                ),
                (
                    "server/controllers/OrdersErrorHandler.go",
                    "// @ErrorHandler\ntype OrdersErrorHandler struct{}\n\n// @HandlesError\n// @ResponseStatus(409)\nfunc (handler *OrdersErrorHandler) Caught(failure error) Caught {\n\treturn Caught{Code: \"caught\"}\n}\n",
                ),
            ],
        },
        Self {
            extension: "py",
            files: &[
                (
                    "server/models/Caught.py",
                    "from dataclasses import dataclass\n\n\n@dataclass\nclass Caught:\n    code: str\n",
                ),
                (
                    "server/controllers/OrdersErrorHandler.py",
                    "from models import Caught\n\n\n@ErrorHandler\nclass OrdersErrorHandler:\n    @HandlesError\n    @ResponseStatus(409)\n    def caught(self, error: Exception) -> Caught:\n        return Caught(\"caught\")\n",
                ),
            ],
        },
    ];

    fn named(extension: &str) -> &'static Self {
        Self::ALL
            .iter()
            .find(|language| language.extension == extension)
            .unwrap_or_else(|| panic!("no root handler for .{extension}"))
    }

    fn validation_examples() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../website/content/docs/annotations/validation-yon/server")
    }

    fn write_project(&self, root: &Path) {
        for role in ["models", "services", "controllers"] {
            let target = root.join("server").join(role);
            std::fs::create_dir_all(&target).expect("role folder");
            for entry in
                std::fs::read_dir(Self::validation_examples().join(role)).expect("examples")
            {
                let path = entry.expect("example").path();
                if path
                    .extension()
                    .is_some_and(|found| found == self.extension)
                {
                    let name = path.file_name().expect("file name");
                    std::fs::copy(&path, target.join(name)).expect("copy example");
                }
            }
        }
        for (relative, source) in self.files {
            std::fs::write(root.join(relative), source).expect("handler");
        }
    }
}

async fn a_root_type_method_leaves_the_check_answer_in(extension: &str) {
    let language = ShapeLanguage::named(extension);
    if !language.available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    RootHandler::named(extension).write_project(root.path());
    let answers = JsonHttp::post_all(root.path(), "/orders", &[r#"{"name":"","level":3}"#]).await;
    let answer = &answers[0];
    assert!(
        answer.starts_with("HTTP/1.1 422 Unprocessable Entity"),
        "{extension}: {answer}"
    );
    assert!(
        answer.contains(r#""errors":[{"field":"name","rule":"NotEmpty"}"#),
        "{extension}: {answer}"
    );
}

one_test_per_case!(
    a_root_type_error_method_does_not_take_the_check_error,
    async a_root_type_method_leaves_the_check_answer_in,
    all RootHandler::ALL.map(|language| language.extension),
    [
        javascript => "js",
        go => "go",
        python => "py",
    ]
);

struct FailingHandler;

impl FailingHandler {
    const FILES: [(&'static str, &'static str); 3] = [
        ("server/roles/Admin.js", "export default class Admin {}\n"),
        (
            "server/controllers/OrdersController.js",
            "@RequestRoute(\"/orders\")\n@SignedIn\nexport class OrdersController {\n  /** @returns {string[]} */\n  @GetRoute\n  getOrders() {\n    return []\n  }\n}\n",
        ),
        (
            "server/controllers/OrdersErrorHandler.js",
            "@ErrorHandler\nexport class OrdersErrorHandler {\n  /**\n   * @param {NotSignedIn} error\n   * @returns {{ code: string }}\n   */\n  @HandlesError\n  signInNeeded(error) {\n    throw new Error('the handler is broken')\n  }\n}\n",
        ),
    ];

    fn write_project(root: &Path) {
        let identity = ErrorShapes::examples().join("controllers/AppIdentityProvider.js");
        std::fs::create_dir_all(root.join("server/controllers")).expect("controllers");
        std::fs::create_dir_all(root.join("server/roles")).expect("roles");
        std::fs::copy(
            identity,
            root.join("server/controllers/AppIdentityProvider.js"),
        )
        .expect("identity provider");
        for (relative, source) in Self::FILES {
            std::fs::write(root.join(relative), source).expect("source");
        }
    }
}

#[tokio::test]
async fn a_failing_error_handler_leaves_the_plain_answer() {
    if !ShapeLanguage::named("js").available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    FailingHandler::write_project(root.path());
    let answers = JsonHttp::get_all(root.path(), &["/orders"]).await;
    let answer = &answers[0];
    assert!(answer.starts_with("HTTP/1.1 401 Unauthorized"), "{answer}");
    assert!(
        answer.ends_with("\r\n\r\nSign in to call this route."),
        "{answer}"
    );
}
