#![allow(clippy::expect_used)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tachyon_contracts::{HandlerRequest, HttpMethod};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions, MigrationFix,
};

use crate::support::authored_behaviors;

use crate::support::behavior_layout;

struct OldNames;

impl OldNames {
    const MARKERS: [(&str, &str); 37] = [
        ("NeedsAll", "RequiredArgsConstructor"),
        ("Named", "Qualifier"),
        ("Lifetime", "Scope"),
        ("Store", "Repository"),
        ("Converter", "Mapper"),
        ("Convert", "Mapping"),
        ("Setup", "Configuration"),
        ("Settings", "ConfigurationProperties"),
        ("Setting", "Value"),
        ("WhenProfile", "Profile"),
        ("WhenSetting", "ConditionalOnProperty"),
        ("OnStart", "PostConstruct"),
        ("OnStop", "PreDestroy"),
        ("OnEvent", "EventListener"),
        ("InBackground", "Async"),
        ("Schedule", "Scheduled"),
        ("Cached", "Cacheable"),
        ("UpdateCache", "CachePut"),
        ("ClearCache", "CacheEvict"),
        ("CacheSettings", "CacheConfig"),
        ("Retry", "Retryable"),
        ("MaxAtOnce", "ConcurrencyLimit"),
        ("AllOrNothing", "Transactional"),
        ("PageDefaults", "PageableDefault"),
        ("WebCall", "HttpExchange"),
        ("Checked", "Validated"),
        ("Check", "Valid"),
        ("Required", "NotNull"),
        ("NotEmpty", "NotBlank"),
        ("Length", "Size"),
        ("Test", "TachyonTest"),
        ("TestSetup", "TestConfiguration"),
        ("Fake", "MockBean"),
        ("UseProfiles", "ActiveProfiles"),
        ("TestSettings", "TestPropertySource"),
        ("FreshStart", "DirtiesContext"),
        ("Needs", "Autowired"),
    ];
    const FOLDERS: [(&str, &str, &str, &str); 4] = [
        ("remotes", "clients", "Client", "Client"),
        ("stores", "repositories", "Store", "Repository"),
        ("setups", "configurations", "Setup", "Configuration"),
        ("converters", "mappers", "Converter", "Mapper"),
    ];

    fn snake(pascal: &str) -> String {
        let mut result = String::new();
        for (index, character) in pascal.chars().enumerate() {
            if character.is_ascii_uppercase() && index > 0 {
                result.push('_');
            }
            result.push(character.to_ascii_lowercase());
        }
        result
    }

    fn lower_camel(name: &str) -> String {
        let mut characters = name.chars();
        let first = characters.next().unwrap_or_default().to_ascii_lowercase();
        format!("{first}{}", characters.as_str())
    }

    fn old_marker(written: &str, has_arguments: bool) -> Option<String> {
        let normalized = written.replace('_', "").to_ascii_lowercase();
        let (new, old) = Self::MARKERS
            .iter()
            .find(|(new, _)| new.to_ascii_lowercase() == normalized)?;
        let old = if *new == "Needs" && has_arguments {
            "Inject"
        } else {
            old
        };
        if written.starts_with(|character: char| character.is_ascii_uppercase()) {
            return Some(String::from(old));
        }
        Some(Self::snake(old))
    }

    fn age_line(line: &str) -> String {
        let indent = &line[..line.len() - line.trim_start().len()];
        let trimmed = line.trim_start();
        for prefix in ["#[yon::", "#[tac::", "@", "#[", "["] {
            let Some(rest) = trimmed.strip_prefix(prefix) else {
                continue;
            };
            let length = rest
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
            let tail = &rest[length..];
            let has_arguments = tail.starts_with('(') && !tail.starts_with("()");
            let Some(old) = Self::old_marker(&rest[..length], has_arguments) else {
                return String::from(line);
            };
            return format!("{indent}{prefix}{old}{tail}");
        }
        String::from(line)
    }

    fn age_markers(source: &str) -> String {
        let mut result = String::new();
        for line in source.split_inclusive('\n') {
            let text = line.trim_end_matches('\n');
            result.push_str(&Self::age_line(text));
            if line.ends_with('\n') {
                result.push('\n');
            }
        }
        result
    }

    fn age_words(source: &str, types: &[(String, String)]) -> String {
        let mut result = String::from(source);
        for (new, old) in types {
            for (from, to) in [
                (new.clone(), old.clone()),
                (Self::lower_camel(new), Self::lower_camel(old)),
                (Self::snake(new), Self::snake(old)),
            ] {
                result = Self::replace_word(&result, &from, &to);
            }
        }
        result
    }

    fn replace_word(source: &str, from: &str, to: &str) -> String {
        let bytes = source.as_bytes();
        let is_word = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$');
        let mut result = String::new();
        let mut cursor = 0;
        for (at, _) in source.match_indices(from) {
            let end = at + from.len();
            let starts = at == 0 || !is_word(bytes[at - 1]);
            let ends = end == bytes.len() || !is_word(bytes[end]);
            if starts && ends && at >= cursor {
                result.push_str(&source[cursor..at]);
                result.push_str(to);
                cursor = end;
            }
        }
        result.push_str(&source[cursor..]);
        result
    }

    fn age_project(root: &Path) {
        let mut types = Vec::new();
        for (new_folder, old_folder, new_suffix, old_suffix) in Self::FOLDERS {
            let directory = root.join("server").join(new_folder);
            let Ok(entries) = fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries {
                let path = entry.expect("entry").path();
                let file = path
                    .file_name()
                    .expect("name")
                    .to_string_lossy()
                    .into_owned();
                let (stem, extension) = file.split_once('.').expect("extension");
                let old_stem = stem.strip_suffix(new_suffix).map_or_else(
                    || String::from(stem),
                    |prefix| format!("{prefix}{old_suffix}"),
                );
                if old_stem != stem {
                    types.push((String::from(stem), old_stem.clone()));
                }
                let target = root
                    .join("server")
                    .join(old_folder)
                    .join(format!("{old_stem}.{extension}"));
                fs::create_dir_all(target.parent().expect("parent")).expect("old folder");
                fs::rename(&path, &target).expect("move back");
            }
            fs::remove_dir(&directory).expect("empty new folder");
        }
        for (relative, source) in snapshot(root) {
            let aged = Self::age_markers(&Self::age_words(&source, &types));
            fs::write(root.join(relative), aged).expect("aged source");
        }
    }
}

fn snapshot(root: &Path) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).expect("directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .expect("inside")
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(relative, fs::read_to_string(&path).expect("text"));
        }
    }
    files
}

fn write_current_project(root: &Path, language: &str) -> String {
    let extension = language;
    authored_behaviors::write(root, language, "server", extension);
    let entry = format!("server/controllers/ProductsController.{extension}");
    if language == "js" {
        behavior_layout::write(
            root,
            &entry,
            "\n@RequestRoute(\"/products\")\n@Public\nclass ProductsController {\n@GetRoute()\nasync get() { return BehaviorChecksHelper.exerciseBehaviors(new AbortController().signal); }\n}",
        );
    }
    entry
}

fn migrate(root: &Path) {
    let plan = MigrationFix::plan(root).expect("plan");
    assert!(plan.change_count() > 0, "{}", plan.to_text(false));
    plan.write(root).expect("write");
    let again = MigrationFix::plan(root).expect("second plan");
    assert_eq!(
        again.change_count() + again.move_count(),
        0,
        "{}",
        again.to_text(false)
    );
}

#[test]
fn a_project_written_with_the_old_names_migrates_back_to_the_current_fixtures() {
    for language in ["js", "py", "php", "java", "kt", "rs", "cs", "go"] {
        let current = tempfile::tempdir().expect("current");
        let entry = write_current_project(current.path(), language);
        let old = tempfile::tempdir().expect("old");
        for (relative, source) in snapshot(current.path()) {
            let target = old.path().join(&relative);
            fs::create_dir_all(target.parent().expect("parent")).expect("folder");
            fs::write(target, source).expect("copy");
        }
        OldNames::age_project(old.path());
        let refused = HandlerSource::discover(old.path(), &entry).expect_err(language);
        let codes: Vec<String> = refused
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.to_string())
            .collect();
        assert!(
            codes
                .iter()
                .any(|code| code == "TY1308" || code == "TY1310"),
            "{language}: {refused}"
        );
        migrate(old.path());
        assert_eq!(snapshot(old.path()), snapshot(current.path()), "{language}");
    }
}

#[tokio::test]
async fn the_migrated_javascript_project_runs_its_behaviors() {
    let current = tempfile::tempdir().expect("current");
    let entry = write_current_project(current.path(), "js");
    OldNames::age_project(current.path());
    migrate(current.path());
    let source = HandlerSource::discover(current.path(), &entry).expect("discovery");
    let supervisor =
        HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
    let response = supervisor
        .invoke(
            &source,
            &HandlerRequest::route("behavior_1", "/products", HttpMethod::Get),
            &HandlerCancellation::default(),
        )
        .await
        .expect("invocation");
    assert!(response.error.is_none(), "{:?}", response.error);
    let body: serde_json::Value =
        serde_json::from_str(&response.body.expect("body").data).expect("JSON");
    assert_eq!(body["first"], body["cached"]);
    assert_eq!(body["saved"]["operation"], "save");
    assert_eq!(body["configuration"]["calls"], 1);
    assert_eq!(body["validation"]["calls"], 1);
}
