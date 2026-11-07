#![allow(clippy::expect_used)]

use super::fsharp_tests::{run_driver, write};
use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::Path;
use std::process::Command;

struct NamedNeedsCase {
    language: WebLanguage,
    extension: &'static str,
    files: &'static [(&'static str, &'static str)],
}

struct NamedNeedsFixtures;

impl NamedNeedsFixtures {
    const KOTLIN: NamedNeedsCase = NamedNeedsCase {
        language: WebLanguage::Kotlin,
        extension: "kt",
        files: &[
            (
                "client/delegates/GreeterDelegate.kt",
                "@Delegate\nclass GreeterDelegate {\n    fun greet(): String = \"hi\"\n}\n",
            ),
            (
                "client/delegates/LoudGreeterDelegate.kt",
                "@Delegate\nclass LoudGreeterDelegate {\n    fun greet(): String = \"HI\"\n}\n",
            ),
            (
                "client/services/WelcomeService.kt",
                "@Service\nclass WelcomeService {\n    @Needs\n    @Named(\"LoudGreeterDelegate\")\n    val greeter: LoudGreeterDelegate\n\n    fun welcome(): String = greeter.greet()\n}\n",
            ),
            (
                "client/services/HelloService.kt",
                "@Service\nclass HelloService {\n    @Needs\n    val greeter: GreeterDelegate\n\n    fun hello(): String = greeter.greet()\n}\n",
            ),
        ],
    };

    const CSHARP: NamedNeedsCase = NamedNeedsCase {
        language: WebLanguage::CSharp,
        extension: "cs",
        files: &[
            (
                "client/delegates/GreeterDelegate.cs",
                "[Delegate]\npublic sealed class GreeterDelegate {\n    public string Greet() {\n        return \"hi\";\n    }\n}\n",
            ),
            (
                "client/delegates/LoudGreeterDelegate.cs",
                "[Delegate]\npublic sealed class LoudGreeterDelegate {\n    public string Greet() {\n        return \"HI\";\n    }\n}\n",
            ),
            (
                "client/services/WelcomeService.cs",
                "[Service]\npublic sealed class WelcomeService {\n    [Needs]\n    [Named(\"LoudGreeterDelegate\")]\n    private readonly LoudGreeterDelegate _greeter;\n\n    \
                 public string Welcome() {\n        return _greeter.Greet();\n    }\n}\n",
            ),
            (
                "client/services/HelloService.cs",
                "[Service]\npublic sealed class HelloService {\n    [Needs]\n    private readonly GreeterDelegate _greeter;\n\n    \
                 public string Hello() {\n        return _greeter.Greet();\n    }\n}\n",
            ),
        ],
    };

    const GO: NamedNeedsCase = NamedNeedsCase {
        language: WebLanguage::Go,
        extension: "go",
        files: &[
            (
                "client/delegates/GreeterDelegate.go",
                "// @Delegate\ntype GreeterDelegate struct {\n\tasked int\n}\n\nfunc (greeter *GreeterDelegate) Greet() string {\n\tgreeter.asked++\n\treturn \"hi\"\n}\n",
            ),
            (
                "client/delegates/LoudGreeterDelegate.go",
                "// @Delegate\ntype LoudGreeterDelegate struct {\n\tasked int\n}\n\nfunc (greeter *LoudGreeterDelegate) Greet() string {\n\tgreeter.asked++\n\treturn \"HI\"\n}\n",
            ),
            (
                "client/services/WelcomeService.go",
                "// @Service\ntype WelcomeService struct {\n\t// @Needs\n\t// @Named(\"LoudGreeterDelegate\")\n\tgreeter *LoudGreeterDelegate\n}\n\n\
                 func (welcome *WelcomeService) Welcome() string {\n\treturn welcome.greeter.Greet()\n}\n",
            ),
            (
                "client/services/HelloService.go",
                "// @Service\ntype HelloService struct {\n\t// @Needs\n\tgreeter *GreeterDelegate\n}\n\n\
                 func (hello *HelloService) Hello() string {\n\treturn hello.greeter.Greet()\n}\n",
            ),
        ],
    };

    const RUST: NamedNeedsCase = NamedNeedsCase {
        language: WebLanguage::Rust,
        extension: "rs",
        files: &[
            (
                "client/delegates/GreeterDelegate.rs",
                "#[Delegate]\nstruct GreeterDelegate {\n    asked: i64,\n}\nimpl GreeterDelegate {\n    fn new() -> Self { Self { asked: 0 } }\n    pub fn greet(&mut self) -> String {\n        self.asked += 1;\n        String::from(\"hi\")\n    }\n}\n",
            ),
            (
                "client/delegates/LoudGreeterDelegate.rs",
                "#[Delegate]\nstruct LoudGreeterDelegate {\n    asked: i64,\n}\nimpl LoudGreeterDelegate {\n    fn new() -> Self { Self { asked: 0 } }\n    pub fn greet(&mut self) -> String {\n        self.asked += 1;\n        String::from(\"HI\")\n    }\n}\n",
            ),
            (
                "client/services/WelcomeService.rs",
                "#[Service]\nstruct WelcomeService {\n    #[Needs]\n    #[Named(\"LoudGreeterDelegate\")]\n    greeter: LoudGreeterDelegate,\n}\nimpl WelcomeService {\n    pub fn welcome(&mut self) -> String {\n        self.greeter.greet()\n    }\n}\n",
            ),
            (
                "client/services/HelloService.rs",
                "#[Service]\nstruct HelloService {\n    #[Needs]\n    greeter: GreeterDelegate,\n}\nimpl HelloService {\n    pub fn hello(&mut self) -> String {\n        self.greeter.greet()\n    }\n}\n",
            ),
        ],
    };

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })\n\
         const open = async (name) => {\n\
         \x20 const module = await import(`./${name}Service.js`)\n\
         \x20 const owner = new module.default({})\n\
         \x20 bus.attach(owner, module.__tachyon_event_plan__.types[0], { applicationEventPublisher: null })\n\
         \x20 await bus.initialize(owner)\n\
         \x20 return owner\n\
         }\n\
         const welcome = await open('Welcome'), hello = await open('Hello')\n\
         const report = { welcomed: welcome.welcome(), greeted: hello.hello() }\n\
         bus.dispose(welcome); bus.dispose(hello)\n\
         console.log(JSON.stringify(report))\n\
         process.exit(0)\n";

    fn runs(program: &str, flag: &str) -> bool {
        Command::new(program)
            .arg(flag)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn ready(case: &NamedNeedsCase) -> bool {
        let compiler = match case.extension {
            "kt" => super::kotlin_tests::kotlin_web_ready(),
            "cs" => super::fsharp_tests::fsharp_web_ready(),
            "rs" => Self::has_rust_web_target(),
            _ => Self::runs("go", "version"),
        };
        compiler && NodeDriver::available()
    }

    fn has_rust_web_target() -> bool {
        Command::new("rustc")
            .args([
                "--print",
                "target-libdir",
                "--target",
                crate::native::RustCompanion::WEB_TARGET,
            ])
            .output()
            .is_ok_and(|output| {
                output.status.success()
                    && Path::new(String::from_utf8_lossy(&output.stdout).trim()).is_dir()
            })
    }

    async fn publish(root: &Path, case: &NamedNeedsCase) {
        let mut services = Vec::new();
        for (relative, source) in case.files.iter().copied() {
            let path = write(root, relative, source);
            let suffix = format!(".{}", case.extension);
            let Some(class) = relative
                .strip_prefix("client/services/")
                .and_then(|name| name.strip_suffix(suffix.as_str()))
            else {
                continue;
            };
            let instance = class.trim_end_matches("Service").to_ascii_lowercase();
            services.push((instance, String::from(class), case.language, path));
        }
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .unwrap_or_else(|failure| panic!("{}: {failure}", case.extension));
        for (name, bytes) in built.files {
            write(root, &name, "");
            std::fs::write(root.join(name), bytes).expect("publish");
        }
        write(
            root,
            "application-events.mjs",
            concat!(
                include_str!("../template/application-events.js"),
                "\nexport default TacApplicationEvents\n"
            ),
        );
    }
}

#[tokio::test]
async fn a_named_needs_field_gets_the_helper_it_names_in_kotlin_csharp_go_and_rust() {
    for case in [
        NamedNeedsFixtures::KOTLIN,
        NamedNeedsFixtures::CSHARP,
        NamedNeedsFixtures::GO,
        NamedNeedsFixtures::RUST,
    ] {
        if !NamedNeedsFixtures::ready(&case) {
            eprintln!(
                "skipping {}: its web compiler or node is unavailable",
                case.extension
            );
            continue;
        }
        let root = tempfile::tempdir().expect("project");
        NamedNeedsFixtures::publish(root.path(), &case).await;
        let report = run_driver(root.path(), NamedNeedsFixtures::DRIVER);
        assert_eq!(
            report,
            serde_json::json!({"welcomed": "HI", "greeted": "hi"}),
            "{}",
            case.extension
        );
    }
}
