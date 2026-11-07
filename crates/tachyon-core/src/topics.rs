use crate::Failure;
use crate::annotation_rules::Realm;
use crate::failure::{diagnostic, source_span};
use crate::topic_access::TopicAccess;
use crate::yon::route_access::RouteAccess;
use std::ops::Range;
use std::path::{Component, Path};

pub(crate) struct Topics;

impl Topics {
    const SPELLINGS: [&'static str; 9] = [
        "@Topic",
        "@Topic()",
        "@topic",
        "@topic()",
        "#[Topic]",
        "#[Topic()]",
        "[Topic]",
        "[<Topic>]",
        "[<Topic()>]",
    ];
    const MAX_SLUG_BYTES: usize = 64;
    const CODE: u16 = 1308;
    const MISSING_CODE: u16 = 1322;
    const ANNOTATION: &'static str = "Topic";
    const HELP: &'static str = "Write @Topic on its own line above a model in a models folder, for example 'server/models/OrderPlaced.py'. JavaScript writes it inside the model's @typedef block (ADR 0061).";
    const MISSING_HELP: &'static str = "Tachyon refuses every topic stream by default (ADR 0061, ADR 0068). Next to @Topic, write @Public when anyone may read the topic stream, @SignedIn when any signed-in user may read it, or @Allow(Role) when only some roles may read it.";
    const ACCESS_HELP: &'static str = "Write one of @Public, @SignedIn or @Allow(Role) next to @Topic on the server model, in the spelling of its language: for example `// @SignedIn` in Go, `/** @SignedIn */` in TypeScript, `#[SignedIn]` in Rust, `[SignedIn]` in C# and `@signed_in` in Python (ADR 0061).";
    const CLIENT_ACCESS: &'static str = "A page cannot decide who reads a topic. Write the access rule on the server model of this topic, in server/models";

    pub(crate) fn markers(source: &str) -> Vec<Range<usize>> {
        let mut offset = 0;
        source
            .split_inclusive('\n')
            .filter_map(|line| {
                let start = offset;
                offset += line.len();
                Self::marker_in(line).map(|marker| start + marker.start..start + marker.end)
            })
            .collect()
    }

    fn marker_in(line: &str) -> Option<Range<usize>> {
        let text = TopicAccess::uncommented(line);
        Self::SPELLINGS.contains(&text).then_some(())?;
        let start = line.find(text)?;
        Some(start..start + text.len())
    }

    pub(crate) fn blanked(source: &str) -> String {
        let mut bytes = source.as_bytes().to_vec();
        Self::markers(source)
            .into_iter()
            .flat_map(|marker| {
                let access = TopicAccess::spans(source, &marker);
                std::iter::once(marker).chain(access)
            })
            .for_each(|span| bytes[span].fill(b' '));
        String::from_utf8(bytes).unwrap_or_else(|_| source.to_owned())
    }

    pub(crate) fn named(source: &str, declared: &[String]) -> Vec<String> {
        Self::markers(source)
            .into_iter()
            .filter_map(|marker| Self::next_declared(&source[marker.end..], declared))
            .collect()
    }

    pub(crate) fn rules(
        path: &Path,
        source: &str,
        declared: &[String],
    ) -> Vec<(String, RouteAccess)> {
        let extension = Self::extension(path);
        Self::markers(source)
            .into_iter()
            .filter_map(|marker| Self::rule_of(extension, source, &marker, declared))
            .collect()
    }

    fn rule_of(
        extension: &str,
        source: &str,
        marker: &Range<usize>,
        declared: &[String],
    ) -> Option<(String, RouteAccess)> {
        let name = Self::next_declared(&source[marker.end..], declared)?;
        let access = TopicAccess::rule(extension, source, marker).ok()?;
        Some((name, access))
    }

    fn extension(path: &Path) -> &str {
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
    }

    fn next_declared(rest: &str, declared: &[String]) -> Option<String> {
        let mut first: Option<(usize, &String)> = None;
        for name in declared {
            let Some(at) = Self::word_at(rest, name) else {
                continue;
            };
            if first.is_none_or(|(earliest, _)| at < earliest) {
                first = Some((at, name));
            }
        }
        first.map(|(_, name)| name.clone())
    }

    fn word_at(text: &str, word: &str) -> Option<usize> {
        let is_word = |character: char| character.is_ascii_alphanumeric() || character == '_';
        for (at, _) in text.match_indices(word) {
            let before = text[..at].chars().next_back();
            let after = text[at + word.len()..].chars().next();
            if !before.is_some_and(is_word) && !after.is_some_and(is_word) {
                return Some(at);
            }
        }
        None
    }

    pub(crate) fn slug(name: &str) -> Option<String> {
        let mut characters = name.chars();
        let first = characters.next()?;
        if !first.is_ascii_uppercase() {
            return None;
        }
        let mut slug = String::from(first.to_ascii_lowercase());
        for character in characters {
            if character.is_ascii_uppercase() {
                slug.push('-');
                slug.push(character.to_ascii_lowercase());
            } else if character.is_ascii_lowercase() || character.is_ascii_digit() {
                slug.push(character);
            } else {
                return None;
            }
        }
        (slug.len() <= Self::MAX_SLUG_BYTES).then_some(slug)
    }

    pub(crate) fn check(path: &Path, source: &str, declared: &[String]) -> Result<(), Failure> {
        let markers = Self::markers(source);
        let Some(first) = markers.first() else {
            return Ok(());
        };
        if let Some(refusal) = Self::refusal(path) {
            return Err(Self::refused(path, source, first.start, &refusal));
        }
        let is_javascript = path.extension().is_some_and(|extension| extension == "js");
        if !is_javascript && !Self::in_models_folder(path) {
            return Err(Self::failure(
                path,
                source,
                first.start,
                "Topic goes on a model in a models folder",
            ));
        }
        for marker in &markers {
            let Some(name) = Self::next_declared(&source[marker.end..], declared) else {
                return Err(Self::failure(
                    path,
                    source,
                    marker.start,
                    "Topic must be written above a model declaration",
                ));
            };
            if Self::slug(&name).is_none() {
                return Err(Self::failure(
                    path,
                    source,
                    marker.start,
                    "A Topic model name starts with a capital letter, uses only letters and digits, and is short (its topic name is at most 64 bytes)",
                ));
            }
            Self::check_access(path, source, marker, &name)?;
        }
        Ok(())
    }

    fn check_access(
        path: &Path,
        source: &str,
        marker: &Range<usize>,
        name: &str,
    ) -> Result<(), Failure> {
        let rule = TopicAccess::rule(Self::extension(path), source, marker)
            .map_err(|(at, message)| Self::access_failure(path, source, at, message))?;
        match (Realm::of(path), rule) {
            (Some(Realm::Yon), RouteAccess::Missing) => {
                Err(Self::missing(path, source, marker.start, name))
            }
            (Some(Realm::Tac), RouteAccess::Missing) | (Some(Realm::Yon) | None, _) => Ok(()),
            (Some(Realm::Tac), _) => {
                let at = TopicAccess::spans(source, marker)
                    .first()
                    .map_or(marker.start, |span| span.start);
                Err(Self::access_failure(path, source, at, Self::CLIENT_ACCESS))
            }
        }
    }

    fn missing(path: &Path, source: &str, at: usize, name: &str) -> Failure {
        let message = format!("The topic {name} does not say who may read its stream");
        Self::located(
            (Self::MISSING_CODE, Self::MISSING_HELP),
            path,
            source,
            at,
            &message,
        )
    }

    fn access_failure(path: &Path, source: &str, at: usize, message: &str) -> Failure {
        Self::located((Self::CODE, Self::ACCESS_HELP), path, source, at, message)
    }

    fn in_models_folder(path: &Path) -> bool {
        let mut components = path.components();
        let realm = components.next();
        let folder = components.next();
        let is_realm =
            matches!(realm, Some(Component::Normal(name)) if name == "server" || name == "client");
        let is_models = matches!(folder, Some(Component::Normal(name)) if name == "models");
        is_realm && is_models
    }

    fn refusal(path: &Path) -> Option<String> {
        let language = path.extension()?.to_str()?;
        let realm = crate::annotation_rules::Realm::of(path)?;
        crate::annotation_rules::AnnotationRules::checked_refusal(Self::ANNOTATION, language, realm)
    }

    fn refused(path: &Path, source: &str, at: usize, refusal: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            Self::CODE,
            format!("{refusal} ({portable})"),
            None,
            source_span(&portable, at, (at + 1).min(source.len())),
        ))
    }

    fn failure(path: &Path, source: &str, at: usize, message: &str) -> Failure {
        Self::located((Self::CODE, Self::HELP), path, source, at, message)
    }

    fn located(
        (code, help): (u16, &str),
        path: &Path,
        source: &str,
        at: usize,
        message: &str,
    ) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        let line = source[..at.min(source.len())].matches('\n').count() + 1;
        Failure::one(diagnostic(
            code,
            format!("{message} ({portable}:{line})"),
            Some(String::from(help)),
            source_span(&portable, at, (at + 1).min(source.len())),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::Topics;
    use crate::yon::route_access::RouteAccess;
    use std::path::Path;

    fn declared(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn every_language_spelling_names_the_next_model() {
        let cases = [
            "@Topic\n@dataclass\nclass OrderPlaced:\n    customer: Customer\n",
            "@Topic\npublic record OrderPlaced(Customer customer) {}\n",
            "#[Topic]\npub struct OrderPlaced { pub customer: Customer }\n",
            "[Topic]\npublic record OrderPlaced(Customer Customer);\n",
            "// @Topic\ntype OrderPlaced struct { Customer Customer }\n",
            "/** @Topic */\nexport interface OrderPlaced { customer: Customer }\n",
            "/**\n * @Topic\n * @typedef {Object} OrderPlaced\n * @property {Customer} customer\n */\n",
            "#[Topic]\nOrderPlaced = Data.define(:customer)\n",
            "[<Topic>]\ntype OrderPlaced = { Customer: Customer }\n",
            "-- @Topic\n---@class OrderPlaced\n---@field customer Customer\nOrderPlaced = {}\n",
        ];
        for source in cases {
            assert_eq!(
                Topics::named(source, &declared(&["Customer", "OrderPlaced"])),
                vec![String::from("OrderPlaced")],
                "{source}"
            );
        }
    }

    #[test]
    fn a_marker_inside_other_text_is_not_a_topic() {
        let source = "// see @Topic above\nclass OrderPlaced {}\nconst text = \"@Topic\"\n";
        assert!(Topics::named(source, &declared(&["OrderPlaced"])).is_empty());
    }

    #[test]
    fn blanking_keeps_offsets_and_removes_the_marker() {
        for (source, declaration) in [
            (
                "#[Topic]\npub struct OrderPlaced {}\n",
                "pub struct OrderPlaced",
            ),
            (
                "[<Topic>]\ntype OrderPlaced = { Id: string }\n",
                "type OrderPlaced",
            ),
        ] {
            let blanked = Topics::blanked(source);
            assert_eq!(blanked.len(), source.len());
            assert!(!blanked.contains("Topic"));
            assert!(blanked.contains(declaration));
        }
    }

    #[test]
    fn topic_names_become_bounded_slugs() {
        assert_eq!(Topics::slug("OrderPlaced").as_deref(), Some("order-placed"));
        assert_eq!(Topics::slug("Tick2").as_deref(), Some("tick2"));
        assert_eq!(Topics::slug("orderPlaced"), None);
        assert_eq!(Topics::slug("Order_Placed"), None);
        assert_eq!(Topics::slug(&format!("A{}", "b".repeat(64))), None);
    }

    #[test]
    fn a_topic_model_passes_the_models_folder_checks_in_every_language() {
        let cases = [
            (
                "server/models/OrderPlaced.py",
                "from dataclasses import dataclass\n\n@topic\n@public\n@dataclass\nclass OrderPlaced:\n    id: str\n",
            ),
            (
                "server/models/OrderPlaced.java",
                "@Topic\n@SignedIn\npublic record OrderPlaced(String id) {}\n",
            ),
            (
                "server/models/OrderPlaced.kt",
                "@Topic\n@Allow(Admin::class)\ndata class OrderPlaced(val id: String)\n",
            ),
            (
                "server/models/OrderPlaced.cs",
                "[Topic]\n[Allow(typeof(Admin))]\npublic record OrderPlaced(string Id);\n",
            ),
            (
                "server/models/OrderPlaced.rs",
                "#[Topic]\n#[allow(dead_code)]\n#[Public]\npub struct OrderPlaced {\n    pub id: String,\n}\n",
            ),
            (
                "server/models/OrderPlaced.go",
                "package main\n\n// @SignedIn\n// @Topic\ntype OrderPlaced struct {\n\tID string `json:\"id\"`\n}\n",
            ),
            (
                "server/models/OrderPlaced.dart",
                "@Topic\n@Allow(Admin)\nclass OrderPlaced {\n  final String id;\n  OrderPlaced(this.id);\n}\n",
            ),
            (
                "server/models/OrderPlaced.php",
                "<?php\n\n#[Topic]\n#[Allow(Admin::class)]\nclass OrderPlaced\n{\n    public string $id;\n}\n",
            ),
            (
                "server/models/OrderPlaced.ts",
                "/** @Topic */\n/** @SignedIn */\nexport interface OrderPlaced {\n  id: string\n}\n",
            ),
            (
                "server/models/OrderPlaced.rb",
                "#[Topic]\n#[SignedIn]\nOrderPlaced = Data.define(:id)\n",
            ),
            (
                "server/models/OrderPlaced.fs",
                "[<Topic>]\n[<Allow(typeof<Admin>)>]\ntype OrderPlaced = { Id: string }\n",
            ),
            (
                "client/models/OrderPlaced.fs",
                "[<Topic>]\ntype OrderPlaced = { Id: string }\n",
            ),
            (
                "server/models/OrderPlaced.lua",
                "-- @Topic\n-- @Public\n---@class OrderPlaced\n---@field id string\nOrderPlaced = {}\n",
            ),
            (
                "client/models/OrderPlaced.lua",
                "-- @Topic\n---@class OrderPlaced\n---@field id string\nOrderPlaced = {}\n",
            ),
            (
                "client/models/OrderPlaced.py",
                "from dataclasses import dataclass\n\n@topic\n@dataclass\nclass OrderPlaced:\n    id: str\n",
            ),
        ];
        for (path, source) in cases {
            let path = Path::new(path);
            let checked = crate::annotation_layout::validate(path, source);
            assert!(checked.is_ok(), "{path:?}: {checked:?}");
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .expect("extension");
            let declared = crate::route_types::ModelTable::declared_names(extension, source);
            assert_eq!(
                Topics::named(source, &declared),
                vec![String::from("OrderPlaced")],
                "{path:?}"
            );
        }
    }

    #[test]
    fn topic_is_refused_outside_a_models_folder_and_without_a_model() {
        let source = "@Topic\n@public\nclass OrderPlaced:\n    id: str\n";
        let models = declared(&["OrderPlaced"]);
        assert!(Topics::check(Path::new("server/models/OrderPlaced.py"), source, &models).is_ok());
        let outside = Topics::check(Path::new("server/services/Orders.py"), source, &models)
            .expect_err("outside models");
        assert!(format!("{outside:?}").contains("models folder"));
        let alone = Topics::check(Path::new("server/models/Nothing.py"), "@Topic\n", &[])
            .expect_err("no model");
        assert!(format!("{alone:?}").contains("above a model declaration"));
        let lower = "@Topic\n@public\nclass orderPlaced:\n    id: str\n";
        assert!(
            Topics::check(
                Path::new("server/models/x.py"),
                lower,
                &declared(&["orderPlaced"])
            )
            .is_err()
        );
    }

    fn rule(path: &str, source: &str) -> RouteAccess {
        let extension = Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .expect("extension");
        let declared = crate::route_types::ModelTable::declared_names(extension, source);
        let rules = Topics::rules(Path::new(path), source, &declared);
        assert_eq!(rules.len(), 1, "{path}: {rules:?}");
        rules[0].1.clone()
    }

    #[test]
    fn every_yon_language_reads_the_access_rule_next_to_its_topic() {
        let admin = || RouteAccess::Allow(vec![String::from("Admin")]);
        let cases = [
            (
                "server/models/A.py",
                "@topic\n@signed_in\n@dataclass\nclass A:\n    id: str\n",
                RouteAccess::SignedIn,
            ),
            (
                "server/models/A.java",
                "@Allow({Admin.class})\n@Topic\nrecord A(String id) {}\n",
                admin(),
            ),
            (
                "server/models/A.groovy",
                "@Topic\n@Public\nrecord A(String id) {}\n",
                RouteAccess::Public,
            ),
            (
                "server/models/A.kt",
                "@Topic\n@Allow(Admin::class)\ndata class A(val id: String)\n",
                admin(),
            ),
            (
                "server/models/A.cs",
                "[Topic]\n[Allow(typeof(Admin))]\npublic record A(string Id);\n",
                admin(),
            ),
            (
                "server/models/A.fs",
                "[<Topic>]\n[<Allow(typeof<Admin>)>]\ntype A = { Id: string }\n",
                admin(),
            ),
            (
                "server/models/A.rs",
                "#[Topic]\n#[allow(dead_code)]\n#[SignedIn]\npub struct A {\n    pub id: String,\n}\n",
                RouteAccess::SignedIn,
            ),
            (
                "server/models/A.go",
                "package main\n\n// @Topic\n// @Allow(Admin)\ntype A struct {\n\tID string\n}\n",
                admin(),
            ),
            (
                "server/models/A.dart",
                "@Topic\n@SignedIn\nclass A {\n  final String id;\n  A(this.id);\n}\n",
                RouteAccess::SignedIn,
            ),
            (
                "server/models/A.php",
                "<?php\n\n#[Topic]\n#[Allow(Admin::class)]\nclass A\n{\n    public string $id;\n}\n",
                admin(),
            ),
            (
                "server/models/A.ts",
                "/** @Allow(Admin) */\n/** @Topic */\nexport interface A {\n  id: string\n}\n",
                admin(),
            ),
            (
                "server/models/A.js",
                "/**\n * @Topic\n * @Public\n * @typedef {Object} A\n * @property {string} id\n */\nexport {}\n",
                RouteAccess::Public,
            ),
            (
                "server/models/A.rb",
                "# @!attribute [r] id\n#   @return [String]\n#[Topic]\n#[Allow(Admin)]\nA = Data.define(:id)\n",
                admin(),
            ),
            (
                "server/models/A.lua",
                "-- @Topic\n-- @SignedIn\n---@class A\n---@field id string\nA = {}\n",
                RouteAccess::SignedIn,
            ),
        ];
        for (path, source, expected) in cases {
            assert_eq!(rule(path, source), expected, "{path}");
            let checked = crate::annotation_layout::validate(Path::new(path), source);
            assert!(checked.is_ok(), "{path}: {checked:?}");
            let blanked = Topics::blanked(source);
            assert_eq!(blanked.len(), source.len(), "{path}");
            for marker in ["Public", "SignedIn", "signed_in", "Allow("] {
                assert!(!blanked.contains(marker), "{path}: {blanked}");
            }
        }
    }

    #[test]
    fn a_server_topic_without_a_rule_is_refused_in_every_yon_language() {
        let cases = [
            (
                "server/models/A.py",
                "@topic\n@dataclass\nclass A:\n    id: str\n",
            ),
            ("server/models/A.java", "@Topic\nrecord A(String id) {}\n"),
            (
                "server/models/A.rs",
                "#[Topic]\n#[allow(dead_code)]\npub struct A {\n    pub id: String,\n}\n",
            ),
            (
                "server/models/A.go",
                "package main\n\n// @Topic\ntype A struct {\n\tID string\n}\n",
            ),
            (
                "server/models/A.ts",
                "/** @Topic */\nexport interface A {\n  id: string\n}\n",
            ),
            (
                "server/models/A.lua",
                "-- @Topic\n---@class A\n---@field id string\nA = {}\n",
            ),
            ("server/models/A.fs", "[<Topic>]\ntype A = { Id: string }\n"),
            (
                "server/models/A.cs",
                "[Topic]\npublic record A(string Id);\n",
            ),
        ];
        for (path, source) in cases {
            let refused = crate::annotation_layout::validate(Path::new(path), source)
                .expect_err("a topic without a rule");
            let text = refused.to_string();
            assert!(
                text.contains("TY1322") && text.contains("does not say who may read"),
                "{path}: {text}"
            );
        }
    }

    #[test]
    fn a_topic_rule_is_one_rule_with_role_types_and_stays_on_the_server() {
        let models = declared(&["A"]);
        let refused = |path: &str, source: &str| {
            format!(
                "{:?}",
                Topics::check(Path::new(path), source, &models).expect_err(path)
            )
        };
        let two = refused(
            "server/models/A.py",
            "@topic\n@public\n@signed_in\nclass A:\n    id: str\n",
        );
        assert!(two.contains("one access rule"), "{two}");
        let argument = refused(
            "server/models/A.java",
            "@Topic\n@Public(Admin.class)\nrecord A(String id) {}\n",
        );
        assert!(argument.contains("take no arguments"), "{argument}");
        let bare = refused(
            "server/models/A.java",
            "@Topic\n@Allow\nrecord A(String id) {}\n",
        );
        assert!(bare.contains("role types"), "{bare}");
        let text = refused(
            "server/models/A.cs",
            "[Topic]\n[Allow(\"Admin\")]\npublic record A(string Id);\n",
        );
        assert!(text.contains("role types"), "{text}");
        let client = refused(
            "client/models/A.py",
            "@topic\n@public\nclass A:\n    id: str\n",
        );
        assert!(
            client.contains("A page cannot decide who reads a topic"),
            "{client}"
        );
        assert!(
            Topics::check(
                Path::new("client/models/A.py"),
                "@topic\nclass A:\n    id: str\n",
                &models
            )
            .is_ok()
        );
    }
}
