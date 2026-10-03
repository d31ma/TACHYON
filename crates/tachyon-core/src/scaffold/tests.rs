#![allow(clippy::expect_used)]

use super::{
    AppFeature, AppName, AppTarget, Choice, ChoiceAnswers, ChoicePrompt, PageFile, ProjectAddition,
    ProjectChoices, RealmChoice, Scaffold, ScaffoldFailure, TacLanguage, TargetFolder, YonLanguage,
};
use std::fs;
use std::io;
use std::path::Path;

#[test]
fn scaffold_serializes_the_app_name_and_is_complete() {
    let root = tempfile::tempdir().expect("workspace");
    let target = root.path().join("app");
    let result = Scaffold::create(&target, Some("A & <B>")).expect("scaffold");
    let script =
        fs::read_to_string(target.join("client/services/ShellService.js")).expect("script");
    let environment = fs::read_to_string(target.join(".env.example")).expect("environment");
    assert!(script.contains("this.documentTitle.set(\"A & <B>\")"));
    assert_eq!(environment, Scaffold::DEFAULT_ENV);
    assert!(!target.join("db").exists());
    assert!(
        target
            .join("server/controllers/HomeController.js")
            .is_file()
    );
    assert!(!target.join("server/routes").exists());
    let test =
        fs::read_to_string(target.join("client/tests/ShellServiceTest.js")).expect("test unit");
    let configuration = fs::read_to_string(target.join("client/tests/ShellTestSetup.js"))
        .expect("separate test configuration");
    assert!(!test.contains("class ShellTestSetup"));
    assert!(configuration.contains("@TestSetup"));
    assert!(configuration.contains("class ShellTestSetup"));
    let controller_test = fs::read_to_string(target.join("server/tests/HomeControllerTest.js"))
        .expect("controller test unit");
    assert!(controller_test.contains("new HomeController().get()"));
    assert!(target.join("server/tests/HomeTestSetup.js").is_file());
    let application = crate::ApplicationConfig::load_with_environment(
        &target,
        &std::collections::BTreeMap::new(),
    )
    .expect("generated application configuration");
    assert_eq!(application.port(), Some(8080));
    assert!(target.join("client/components/app/hero/tac.html").is_file());
    assert!(!target.join("client/pages/tac.js").exists());
    assert!(!target.join("client/pages/tac.css").exists());
    assert!(!target.join("client/components/app/shell/tac.js").exists());
    assert!(script.contains("@Service"));
    assert!(script.contains("export default class ShellService"));
    assert!(script.contains("brand = 'Tachyon'"));
    assert!(script.contains("@OnStart"));
    for generated in [&script, &test, &configuration] {
        assert!(!generated.contains("tachyon:behaviors"), "{generated}");
    }
    let shell = fs::read_to_string(target.join("client/components/app/shell/tac.html"))
        .expect("shell view");
    assert!(shell.contains("{shell.brand}"));
    assert_eq!(result.app_name(), "A & <B>");
    assert_eq!(result.root(), fs::canonicalize(target).expect("canonical"));
}

#[test]
fn generated_shell_startup_uses_its_injected_document_title_provider() {
    let (_root, app) = generated(&ProjectChoices::defaults());
    let relative = Path::new("client/tests/ShellServiceTest.js");
    let report = crate::client_tests::run(&app, relative).expect("generated shell test runs");
    assert_eq!(report.cases.len(), 1);
    assert!(report.failure.is_none(), "{:?}", report.failure);
    let case = &report.cases[0];
    assert!(case.passed, "{}: {:?}", case.name, case.error);
    let service =
        fs::read_to_string(app.join("client/services/ShellService.js")).expect("shell service");
    assert!(service.contains("@Needs\n  documentTitle"), "{service}");
    assert!(!service.contains("document.title"), "{service}");
    let test = fs::read_to_string(app.join(relative)).expect("shell test");
    assert!(test.contains("@Fake\n  documentTitle"), "{test}");
    assert!(!test.contains("globalThis"), "{test}");
    let manifest = fs::read_to_string(app.join("client/manifest.json")).expect("manifest");
    let manifest: serde_json::Value = serde_json::from_str(&manifest).expect("JSON manifest");
    assert!(manifest["tac"]["browserApis"].is_null(), "{manifest}");
}

#[test]
fn each_realm_has_a_jsconfig_that_reads_its_composed_types() {
    let root = tempfile::tempdir().expect("workspace");
    let target = root.path().join("app");
    Scaffold::create(&target, None).expect("scaffold");
    assert!(!target.join("jsconfig.json").exists());
    for realm in ["client", "server"] {
        let text =
            fs::read_to_string(target.join(realm).join("jsconfig.json")).expect("realm jsconfig");
        let config: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        let include = config["include"].as_array().expect("include list");
        let types = format!("../.tachyon/types/{realm}");
        assert!(include.iter().any(|item| item == types.as_str()), "{text}");
    }
    let ignored = fs::read_to_string(target.join(".gitignore")).expect("gitignore");
    assert!(ignored.lines().any(|line| line == ".tachyon/"), "{ignored}");
    assert!(
        !ignored.lines().any(|line| line.starts_with("data")),
        "{ignored}"
    );
}

#[test]
fn existing_empty_targets_and_default_names_are_supported() {
    let root = tempfile::tempdir().expect("workspace");
    let target = root.path().join("default-name");
    fs::create_dir(&target).expect("empty target");
    let result = Scaffold::create(&target, None).expect("scaffold");
    assert_eq!(result.app_name(), "default-name");
    assert!(target.join("README.md").is_file());
}

#[test]
fn invalid_targets_and_names_fail_before_writing() {
    let root = tempfile::tempdir().expect("workspace");
    let file = root.path().join("file");
    fs::write(&file, "occupied").expect("file");
    assert!(
        Scaffold::create(&file, None)
            .expect_err("file target")
            .to_string()
            .contains("TY1401")
    );

    for name in ["", "\n", &"x".repeat(101)] {
        let target = root.path().join(format!("invalid-{}", name.len()));
        assert!(
            Scaffold::create(&target, Some(name))
                .expect_err("invalid name")
                .to_string()
                .contains("TY1403")
        );
        assert!(!target.exists());
    }

    assert!(TargetFolder::absolute(Path::new("")).is_err());
    assert!(
        TargetFolder::absolute(root.path())
            .expect("absolute")
            .is_absolute()
    );
    assert!(
        TargetFolder::absolute(Path::new("relative"))
            .expect("relative")
            .is_absolute()
    );
    assert_eq!(
        AppName::validated(None, Path::new("/")).expect("fallback"),
        "Tachyon App"
    );

    let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
    assert!(
        ScaffoldFailure::io(&file, &io_error)
            .to_string()
            .contains("TY1401")
    );
    assert!(ScaffoldFailure::checked::<()>(Err(io_error), &file).is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_targets_are_rejected() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("workspace");
    let directory = root.path().join("directory");
    let link = root.path().join("link");
    fs::create_dir(&directory).expect("directory");
    symlink(&directory, &link).expect("symlink");
    assert!(
        Scaffold::create(&link, None)
            .expect_err("symlink")
            .to_string()
            .contains("TY1401")
    );
}

fn answers(realm: Option<&str>, tac: Option<&str>, yon: Option<&str>) -> ChoiceAnswers {
    ChoiceAnswers {
        realm: realm.map(String::from),
        tac: tac.map(String::from),
        yon: yon.map(String::from),
        targets: None,
        features: None,
    }
}

fn error_text(answers: &ChoiceAnswers) -> String {
    ProjectChoices::from_answers(answers)
        .expect_err("refused")
        .to_string()
}

#[test]
fn no_answers_mean_both_realms_javascript_and_the_web() {
    let choices = ProjectChoices::from_answers(&ChoiceAnswers::default()).expect("defaults");
    assert_eq!(choices, ProjectChoices::defaults());
    let server = ProjectChoices::from_answers(&answers(Some("server"), None, Some("python, go")))
        .expect("server");
    assert!(server.tac.is_empty() && server.targets.is_empty());
    assert_eq!(server.yon, vec![YonLanguage::Python, YonLanguage::Go]);
    let client = ProjectChoices::from_answers(&answers(Some("tac"), Some("kt,ts,TS"), None))
        .expect("client");
    assert_eq!(
        client.tac,
        vec![TacLanguage::TypeScript, TacLanguage::Kotlin]
    );
    assert_eq!(client.targets, vec![AppTarget::Web]);
}

#[test]
fn unknown_and_conflicting_answers_are_refused_with_ty1408() {
    let unknown = error_text(&answers(None, Some("cobol"), None));
    assert!(
        unknown.contains("TY1408") && unknown.contains("'cobol' is not a Tac language"),
        "{unknown}"
    );
    assert!(
        unknown.contains("js, ts, dart, rs, cs, fs, go, rb, lua, swift, kt, java"),
        "{unknown}"
    );
    let python_page = error_text(&answers(None, Some("py"), None));
    assert!(
        python_page.contains("'py' is not a Tac language"),
        "{python_page}"
    );
    let no_server = error_text(&answers(Some("client"), None, Some("py")));
    assert!(no_server.contains("no server"), "{no_server}");
    let mut sockets = answers(Some("client"), None, None);
    sockets.features = Some(String::from("sockets"));
    assert!(error_text(&sockets).contains("The sockets example needs a server"));
    let mut targets = answers(Some("server"), None, None);
    targets.targets = Some(String::from("ios"));
    assert!(error_text(&targets).contains("no client"));
    let empty = error_text(&answers(Some("both"), Some(" , "), None));
    assert!(empty.contains("at least one Tac language"), "{empty}");
}

#[test]
fn every_choice_has_a_flag_name_and_a_label() {
    fn check<T: Choice + std::fmt::Debug>() {
        for choice in T::ALL {
            assert!(!choice.flag_name().is_empty());
            assert!(!choice.label().is_empty());
            assert_eq!(T::named(&choice.flag_name().to_uppercase()), Some(*choice));
        }
    }
    check::<RealmChoice>();
    check::<TacLanguage>();
    check::<YonLanguage>();
    check::<AppTarget>();
    check::<AppFeature>();
}

fn generated(choices: &ProjectChoices) -> (tempfile::TempDir, std::path::PathBuf) {
    let root = tempfile::tempdir().expect("workspace");
    let target = root.path().join("app");
    Scaffold::create_with(&target, Some("Wizard"), choices).expect("scaffold");
    (root, target)
}

#[test]
fn only_the_chosen_realms_languages_and_features_are_generated() {
    let choices = ProjectChoices {
        realm: RealmChoice::Both,
        tac: vec![
            TacLanguage::TypeScript,
            TacLanguage::Rust,
            TacLanguage::Swift,
        ],
        yon: vec![YonLanguage::Python, YonLanguage::Php],
        targets: vec![AppTarget::Web, AppTarget::Macos],
        features: vec![AppFeature::Sockets, AppFeature::Storage],
    };
    let (_root, app) = generated(&choices);
    for file in [
        "client/services/TypeScriptGreetingService.ts",
        "client/tests/TypeScriptGreetingServiceTest.ts",
        "client/services/RustGreetingService.rs",
        "client/services/SwiftGreetingService.swift",
        "client/services/SwiftGreetingService.js",
        "client/components/greeting/rust/tac.html",
        "server/controllers/PythonGreetingController.py",
        "server/tests/PhpGreetingControllerTest.php",
        "server/controllers/ChatController.js",
        "server/models/ChatMessage.js",
        "client/services/ChatService.ts",
        "client/remotes/ChatClient.ts",
        "client/models/ChatMessage.ts",
        "server/services/NoteService.js",
    ] {
        assert!(app.join(file).is_file(), "{file}");
    }
    for absent in [
        "server/models/ChatMessage.ts",
        "client/models/ChatMessage.js",
        "client/services/ShellService.js",
        "server/controllers/HomeController.js",
        "client/services/ChatService.js",
        "server/controllers/GoGreetingController.go",
    ] {
        assert!(!app.join(absent).exists(), "{absent}");
    }
    let page = fs::read_to_string(app.join(PageFile::PATH)).expect("page");
    assert!(
        page.contains("<greeting-typescript hydrate=\"load\" />"),
        "{page}"
    );
    assert!(page.contains("<chat-room hydrate=\"load\" />"), "{page}");
    let shell =
        fs::read_to_string(app.join("client/components/app/shell/tac.html")).expect("shell");
    assert!(!shell.contains("{shell.brand}"), "{shell}");
    let rust_view =
        fs::read_to_string(app.join("client/components/greeting/rust/tac.html")).expect("view");
    assert!(
        rust_view.contains("{rustGreeting.message}") && rust_view.contains("rustGreeting.greet()")
    );
    let package = fs::read_to_string(app.join("package.json")).expect("package");
    let package: serde_json::Value = serde_json::from_str(&package).expect("JSON package");
    assert_eq!(
        package["scripts"]["bundle:macos"],
        "ty bundle --target macos"
    );
    assert_eq!(package["devDependencies"]["typescript"], "7.0.2");
    let application = fs::read_to_string(app.join("server/application.json")).expect("application");
    let application: serde_json::Value = serde_json::from_str(&application).expect("JSON");
    assert_eq!(application["datasource"]["driver"], "sqlite");
}

struct ClientStyleFixture;

impl ClientStyleFixture {
    fn language(language: TacLanguage) {
        let choices = ProjectChoices {
            realm: RealmChoice::Both,
            tac: vec![language],
            yon: vec![YonLanguage::JavaScript],
            targets: vec![AppTarget::Web],
            features: AppFeature::ALL.to_vec(),
        };
        let (_root, app) = generated(&choices);
        let manifest = fs::read_to_string(app.join("client/manifest.json")).expect("manifest");
        let manifest: serde_json::Value = serde_json::from_str(&manifest).expect("JSON manifest");
        Self::foundation(&app, &manifest, language);
        Self::grants(&manifest, language);
        Self::greeting(&app, language);
        Self::features(&app, language);
    }

    fn foundation(app: &Path, manifest: &serde_json::Value, language: TacLanguage) {
        assert_eq!(
            manifest["tac"]["styles"],
            serde_json::json!(["/shared/styles/app.css"]),
            "{language:?}"
        );
        assert_eq!(
            manifest["tac"]["styleSources"]["/shared/styles/app.css"]["kind"], "foundation",
            "{language:?}"
        );
        assert!(manifest["tac"]["scripts"].is_null(), "{language:?}");
        assert!(!app.join("client/shared/scripts/imports.js").exists());
        let foundation = fs::read_to_string(app.join("client/shared/styles/app.css"))
            .expect("foundation styles");
        assert!(foundation.contains("color-scheme: dark"), "{foundation}");
        assert!(foundation.contains("background: #0f172a"), "{foundation}");
        assert!(foundation.contains("color: #e2e8f0"), "{foundation}");
        assert!(!foundation.contains(".greeting"), "{foundation}");
        assert!(!foundation.contains("data-theme"), "{foundation}");
        assert!(!app.join("client/pages/tac.css").exists());
    }

    fn grants(manifest: &serde_json::Value, language: TacLanguage) {
        assert!(manifest["tac"]["browserApis"].is_null(), "{language:?}");
    }

    fn greeting(app: &Path, language: TacLanguage) {
        if language == TacLanguage::JavaScript {
            return;
        }
        let component = app.join(format!("client/components/greeting/{}", language.slug()));
        let styles = fs::read_to_string(component.join("tac.css")).expect("greeting styles");
        assert_eq!(
            styles, ".greeting { margin-top: 1.5rem; }\n",
            "{language:?}"
        );
        let view = fs::read_to_string(component.join("tac.html")).expect("greeting view");
        assert!(view.contains("class=\"greeting\""), "{language:?}: {view}");
        assert!(!app.join("client/services/ShellService.js").exists());
    }

    fn features(app: &Path, language: TacLanguage) {
        let page = fs::read_to_string(app.join(PageFile::PATH)).expect("page");
        let tags = AppFeature::ALL
            .iter()
            .copied()
            .filter_map(super::ClientFiles::feature_tag);
        for tag in tags {
            assert!(page.contains(&tag), "{language:?}: {tag}");
        }
    }
}

#[test]
fn javascript_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::JavaScript);
}

#[test]
fn typescript_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::TypeScript);
}

#[test]
fn dart_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::Dart);
}

#[test]
fn rust_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::Rust);
}

#[test]
fn csharp_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::CSharp);
}

#[test]
fn fsharp_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::FSharp);
}

#[test]
fn go_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::Go);
}

#[test]
fn ruby_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::Ruby);
}

#[test]
fn lua_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::Lua);
}

#[test]
fn swift_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::Swift);
}

#[test]
fn kotlin_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::Kotlin);
}

#[test]
fn java_scaffold_keeps_foundation_and_component_styles_separate() {
    ClientStyleFixture::language(TacLanguage::Java);
}

#[test]
fn adding_a_greeting_language_preserves_existing_foundation_styles() {
    let (_root, app) = generated(&ProjectChoices::defaults());
    let foundation_path = app.join("client/shared/styles/app.css");
    let foundation = ":root { color-scheme: light; --brand-color: #112233; }\n";
    fs::write(&foundation_path, foundation).expect("authored foundation");
    let added = ProjectAddition::language(&app, RealmChoice::Client, "go").expect("Go");
    assert!(
        added
            .written
            .contains(&String::from("client/components/greeting/go/tac.css"))
    );
    assert_eq!(
        fs::read_to_string(foundation_path).expect("foundation"),
        foundation
    );
    let styles = fs::read_to_string(app.join("client/components/greeting/go/tac.css"))
        .expect("greeting styles");
    assert_eq!(styles, ".greeting { margin-top: 1.5rem; }\n");
}

#[test]
fn a_client_only_or_server_only_project_has_one_realm() {
    let client =
        ProjectChoices::from_answers(&answers(Some("client"), None, None)).expect("client");
    let (_root, app) = generated(&client);
    assert!(app.join("client/services/ShellService.js").is_file());
    assert!(!app.join("server").exists() && !app.join(".env.example").exists());
    let server =
        ProjectChoices::from_answers(&answers(Some("server"), None, Some("rb"))).expect("server");
    let (_root, app) = generated(&server);
    assert!(
        app.join("server/controllers/RubyGreetingController.rb")
            .is_file()
    );
    assert!(!app.join("client").exists());
    let package = fs::read_to_string(app.join("package.json")).expect("package");
    assert!(!package.contains("devDependencies"), "{package}");
}

#[test]
fn every_language_template_is_one_greeting_class_with_a_test_and_a_setup() {
    for language in TacLanguage::ALL
        .iter()
        .filter(|language| **language != TacLanguage::JavaScript)
    {
        let files = language.files();
        assert!(
            files.iter().any(|(path, _)| path.contains("/services/")),
            "{language:?}"
        );
        assert!(
            files.iter().any(|(path, _)| path.contains("ServiceTest.")),
            "{language:?}"
        );
        assert!(
            files.iter().all(|(_, text)| text.contains("Greeting")),
            "{language:?}"
        );
    }
    for language in YonLanguage::ALL
        .iter()
        .filter(|language| **language != YonLanguage::JavaScript)
    {
        let files = language.files();
        assert_eq!(files.len(), 3, "{language:?}");
        let test = files
            .iter()
            .find(|(path, _)| path.contains("ControllerTest"))
            .expect("test");
        assert!(
            test.1.contains("GreetingController()")
                || test.1.contains("GreetingController{")
                || test.1.contains("GreetingController::new()")
                || test.1.contains("construct("),
            "{language:?}"
        );
    }
}

#[test]
fn a_scripted_prompt_flow_chooses_by_number_and_by_name() {
    let mut input = "\n1\n2, rust\npy,9\n1\n6,1\n".as_bytes();
    let mut output = Vec::new();
    let choices = ChoicePrompt::new(&mut input, &mut output)
        .ask()
        .expect("choices");
    let text = String::from_utf8(output).expect("text");
    assert_eq!(choices.realm, RealmChoice::Both);
    assert_eq!(choices.tac, vec![TacLanguage::JavaScript]);
    assert_eq!(
        choices.yon,
        vec![YonLanguage::TypeScript, YonLanguage::Rust]
    );
    assert_eq!(choices.targets, vec![AppTarget::Web]);
    assert_eq!(
        choices.features,
        vec![AppFeature::Storage, AppFeature::ErrorHandlers]
    );
    assert!(text.contains("1. What should the app have?"), "{text}");
    assert!(
        text.contains("Please type only numbers from the list."),
        "{text}"
    );
    assert!(text.contains("Press Enter for 1: "), "{text}");
}

#[test]
fn a_client_prompt_skips_the_server_questions_and_offers_client_examples_only() {
    let mut input = "2\n2\n1,5\n1\n".as_bytes();
    let mut output = Vec::new();
    let choices = ChoicePrompt::new(&mut input, &mut output)
        .ask()
        .expect("choices");
    let text = String::from_utf8(output).expect("text");
    assert_eq!(choices.realm, RealmChoice::Client);
    assert_eq!(choices.tac, vec![TacLanguage::TypeScript]);
    assert!(choices.yon.is_empty());
    assert_eq!(choices.targets, vec![AppTarget::Web, AppTarget::Ios]);
    assert_eq!(choices.features, vec![AppFeature::HttpClient]);
    assert!(!text.contains("server (Yon)?"), "{text}");
    assert!(!text.contains("Sockets"), "{text}");
}

#[test]
fn a_closed_input_keeps_every_default() {
    let mut input = "".as_bytes();
    let mut output = Vec::new();
    let choices = ChoicePrompt::new(&mut input, &mut output)
        .ask()
        .expect("choices");
    assert_eq!(choices, ProjectChoices::defaults());
}

#[test]
fn add_writes_a_new_language_feature_and_target_without_overwriting() {
    let (_root, app) = generated(&ProjectChoices::defaults());
    let added = ProjectAddition::language(&app, RealmChoice::Server, "py").expect("python");
    assert!(added.written.contains(&String::from(
        "server/controllers/PythonGreetingController.py"
    )));
    assert!(added.choices.yon.contains(&YonLanguage::Python));
    let again = ProjectAddition::language(&app, RealmChoice::Server, "python").expect_err("exists");
    assert!(again.to_string().contains("TY1402"), "{again}");
    let both = ProjectAddition::language(&app, RealmChoice::Both, "py").expect_err("realm");
    assert!(both.to_string().contains("TY1408"), "{both}");
    ProjectAddition::language(&app, RealmChoice::Client, "go").expect("go");
    let page = fs::read_to_string(app.join(PageFile::PATH)).expect("page");
    assert!(page.contains("<greeting-go hydrate=\"load\" />"), "{page}");
    ProjectAddition::feature(&app, AppFeature::Storage).expect("storage");
    let application = fs::read_to_string(app.join("server/application.json")).expect("application");
    assert!(application.contains("sqlite::memory:"), "{application}");
    assert!(application.contains("8080"), "{application}");
    assert!(app.join("server/tests/NoteServiceTest.js").is_file());
    ProjectAddition::target(&app, AppTarget::Android).expect("android");
    let package = fs::read_to_string(app.join("package.json")).expect("package");
    let package: serde_json::Value = serde_json::from_str(&package).expect("JSON package");
    assert_eq!(
        package["scripts"]["bundle:android"],
        "ty bundle --target android"
    );
    let state = ProjectAddition::target(&app, AppTarget::Android).expect("idempotent");
    assert_eq!(state.choices.features, vec![AppFeature::Storage]);
    assert!(state.choices.targets.contains(&AppTarget::Android));
}

#[test]
fn add_to_a_server_only_project_creates_the_client_realm() {
    let server =
        ProjectChoices::from_answers(&answers(Some("server"), None, None)).expect("server");
    let (_root, app) = generated(&server);
    let added = ProjectAddition::language(&app, RealmChoice::Client, "ts").expect("client");
    assert_eq!(added.choices.realm, RealmChoice::Both);
    assert!(app.join("client/manifest.json").is_file());
    assert!(
        app.join("client/services/TypeScriptGreetingService.ts")
            .is_file()
    );
    let page = fs::read_to_string(app.join(PageFile::PATH)).expect("page");
    assert!(page.contains("greeting-typescript"), "{page}");
    let feature = ProjectAddition::feature(
        &tempfile::tempdir().expect("empty").path().join("none"),
        AppFeature::Topics,
    );
    assert!(feature.is_err());
}

#[test]
fn only_a_typescript_realm_keeps_the_field_start_check_of_tsc() {
    let field_check = |target: &std::path::Path, realm: &str| -> serde_json::Value {
        let text =
            fs::read_to_string(target.join(realm).join("jsconfig.json")).expect("realm jsconfig");
        let config: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(config["compilerOptions"]["strict"], true, "{text}");
        config["compilerOptions"]["strictPropertyInitialization"].clone()
    };
    let (_javascript_root, javascript) = generated(&ProjectChoices::defaults());
    let typescript_choices =
        ProjectChoices::from_answers(&answers(None, Some("ts"), Some("ts"))).expect("typescript");
    let (_typescript_root, typescript) = generated(&typescript_choices);
    for realm in ["client", "server"] {
        assert_eq!(field_check(&javascript, realm), false);
        assert!(field_check(&typescript, realm).is_null());
    }
}
