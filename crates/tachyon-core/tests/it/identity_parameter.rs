#![allow(clippy::expect_used)]

use crate::support::json_http::JsonHttp;
use crate::support::one_test_per_case::one_test_per_case;
use crate::support::program_probe::ProgramProbe;
use std::path::{Path, PathBuf};
use tachyon_core::ProjectDiscovery;

struct IdentityLanguage {
    extension: &'static str,
    programs: &'static [&'static str],
}

impl IdentityLanguage {
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

    const ROLES: [&'static str; 3] = ["controllers", "models", "roles"];

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
            .unwrap_or_else(|| panic!("no identity language uses .{extension}"))
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
                "skipping {} identity parameter: missing one of {:?}",
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
            let target = root.join("server").join(role);
            std::fs::create_dir_all(&target).expect("role folder");
            for entry in std::fs::read_dir(IdentityExample::folder().join(role)).expect("examples")
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
    }
}

struct IdentityExample;

impl IdentityExample {
    fn folder() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../website/content/docs/yon/identity-in-every-yon-language/server")
    }

    fn request(token: Option<&str>) -> String {
        let authorization = token
            .map(|token| format!("Authorization: {token}\r\n"))
            .unwrap_or_default();
        format!("GET /me HTTP/1.1\r\nHost: localhost\r\n{authorization}Connection: close\r\n\r\n")
    }

    fn body(response: &str) -> &str {
        response.split_once("\r\n\r\n").map_or("", |(_, body)| body)
    }
}

async fn a_route_receives_the_identity_in(extension: &str) {
    let language = IdentityLanguage::named(extension);
    if !language.available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    language.write_project(root.path());
    let requests = [
        IdentityExample::request(Some("ada-token")),
        IdentityExample::request(Some("bob-token")),
        IdentityExample::request(None),
    ];
    let answers = JsonHttp::exchange_all(root.path(), &requests).await;
    let [ada, bob, nobody] = [&answers[0], &answers[1], &answers[2]];
    assert!(ada.starts_with("HTTP/1.1 200 OK"), "{extension}: {ada}");
    assert_eq!(IdentityExample::body(ada), "\"ada\"", "{extension}: {ada}");
    assert_eq!(IdentityExample::body(bob), "\"bob\"", "{extension}: {bob}");
    assert!(
        nobody.starts_with("HTTP/1.1 401 Unauthorized"),
        "{extension}: {nobody}"
    );
}

one_test_per_case!(
    a_route_method_receives_the_identity_of_the_signed_in_caller,
    async a_route_receives_the_identity_in,
    all IdentityLanguage::ALL.map(|language| language.extension),
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

struct LooseProvider;

impl LooseProvider {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "server/models/Identity.ts",
            "export interface Identity {\n  user: string;\n  roles: string[];\n}\n",
        ),
        (
            "server/models/LooseIdentity.ts",
            "export interface LooseIdentity {\n  user: string;\n  roles: string[];\n  secret: string;\n}\n",
        ),
        (
            "server/controllers/AppIdentityProvider.ts",
            "import type { LooseIdentity } from '/models/LooseIdentity.ts';\n\n@IdentityProvider\nexport class AppIdentityProvider {\n  identify(request: Request): LooseIdentity {\n    return { user: 'ada-secret-user', roles: ['Admin'], secret: 'hunter2' };\n  }\n}\n",
        ),
        (
            "server/controllers/MeController.ts",
            "import type { Identity } from '/models/Identity.ts';\n\n@RequestRoute(\"/me\")\n@SignedIn\nexport class MeController {\n  @GetRoute\n  getMe(@FromIdentity caller: Identity): string {\n    return caller.user;\n  }\n}\n",
        ),
    ];

    fn write_project(root: &Path) {
        for (relative, source) in Self::FILES {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
            std::fs::write(path, source).expect("source");
        }
    }
}

#[tokio::test]
async fn an_identity_that_does_not_fit_its_model_answers_500_without_its_values() {
    if !IdentityLanguage::named("ts").available() {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    LooseProvider::write_project(root.path());
    let answers =
        JsonHttp::exchange_all(root.path(), &[IdentityExample::request(Some("any-token"))]).await;
    let answer = &answers[0];
    assert!(
        answer.starts_with("HTTP/1.1 500 Internal Server Error"),
        "{answer}"
    );
    assert!(
        IdentityExample::body(answer).starts_with("Handler execution failed. Reference: "),
        "{answer}"
    );
    for value in ["ada-secret-user", "hunter2", "secret", "Admin"] {
        assert!(!answer.contains(value), "{value} is in {answer}");
    }
}

struct PublicIdentity;

impl PublicIdentity {
    const IDENTITY: [(&'static str, &'static str); 2] = [
        (
            "server/models/Identity.ts",
            "export interface Identity {\n  user: string;\n  roles: string[];\n}\n",
        ),
        (
            "server/controllers/AppIdentityProvider.ts",
            "import type { Identity } from '/models/Identity.ts';\n\n@IdentityProvider\nexport class AppIdentityProvider {\n  identify(request: Request): Identity | null {\n    return null;\n  }\n}\n",
        ),
    ];
    const PUBLIC_METHOD: &'static str = "import type { Identity } from '/models/Identity.ts';\n\n@RequestRoute(\"/me\")\n@SignedIn\nexport class MeController {\n  @GetRoute\n  @Public\n  getMe(@FromIdentity caller: Identity): string {\n    return caller.user;\n  }\n}\n";
    const SIGNED_IN_METHOD: &'static str = "import type { Identity } from '/models/Identity.ts';\n\n@RequestRoute(\"/me\")\n@Public\nexport class MeController {\n  @GetRoute\n  @SignedIn\n  getMe(@FromIdentity caller: Identity): string {\n    return caller.user;\n  }\n}\n";

    fn write_project(root: &Path, controller: &str) {
        let controllers = root.join("server/controllers");
        std::fs::create_dir_all(&controllers).expect("controllers");
        std::fs::create_dir_all(root.join("server/models")).expect("models");
        for (relative, source) in Self::IDENTITY {
            std::fs::write(root.join(relative), source).expect("source");
        }
        std::fs::write(controllers.join("MeController.ts"), controller).expect("controller");
    }
}

#[test]
fn from_identity_on_a_public_route_is_refused_and_the_method_rule_wins() {
    let refused = tempfile::tempdir().expect("project");
    PublicIdentity::write_project(refused.path(), PublicIdentity::PUBLIC_METHOD);
    let failure = ProjectDiscovery::discover(refused.path()).expect_err("public identity");
    let text = failure.to_string();
    assert!(text.contains("TY1322"), "{text}");
    assert!(
        text.contains("is @Public, but its method getMe takes a @FromIdentity parameter"),
        "{text}"
    );
    let accepted = tempfile::tempdir().expect("project");
    PublicIdentity::write_project(accepted.path(), PublicIdentity::SIGNED_IN_METHOD);
    ProjectDiscovery::discover(accepted.path()).expect("a signed-in method may take the identity");
}
