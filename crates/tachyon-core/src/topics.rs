use crate::Failure;
use crate::failure::{diagnostic, source_span};
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
    const COMMENT_LEADERS: [&'static str; 5] = ["/**", "///", "//", "# ", "*"];
    const MAX_SLUG_BYTES: usize = 64;
    const CODE: u16 = 1308;
    const HELP: &'static str = "Write @Topic on its own line above a model in a models folder, for example 'server/models/OrderPlaced.py'. JavaScript writes it inside the model's @typedef block (ADR 0061).";

    pub(crate) fn markers(source: &str) -> Vec<Range<usize>> {
        let mut found = Vec::new();
        let mut offset = 0;
        for line in source.split_inclusive('\n') {
            if let Some(marker) = Self::marker_in(line) {
                found.push(offset + marker.start..offset + marker.end);
            }
            offset += line.len();
        }
        found
    }

    fn marker_in(line: &str) -> Option<Range<usize>> {
        let mut text = line.trim();
        text = text.strip_suffix("*/").unwrap_or(text).trim_end();
        for leader in Self::COMMENT_LEADERS {
            if let Some(rest) = text.strip_prefix(leader) {
                text = rest.trim_start();
                break;
            }
        }
        if !Self::SPELLINGS.contains(&text) {
            return None;
        }
        let start = line.find(text)?;
        Some(start..start + text.len())
    }

    pub(crate) fn blanked(source: &str) -> String {
        let mut bytes = source.as_bytes().to_vec();
        for marker in Self::markers(source) {
            for byte in &mut bytes[marker] {
                *byte = b' ';
            }
        }
        String::from_utf8(bytes).unwrap_or_else(|_| source.to_owned())
    }

    pub(crate) fn named(source: &str, declared: &[String]) -> Vec<String> {
        let mut names = Vec::new();
        for marker in Self::markers(source) {
            if let Some(name) = Self::next_declared(&source[marker.end..], declared) {
                names.push(name);
            }
        }
        names
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
        }
        Ok(())
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

    fn failure(path: &Path, source: &str, at: usize, message: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        let line = source[..at.min(source.len())].matches('\n').count() + 1;
        Failure::one(diagnostic(
            Self::CODE,
            format!("{message} ({portable}:{line})"),
            Some(String::from(Self::HELP)),
            source_span(&portable, at, (at + 1).min(source.len())),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::Topics;
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
                "from dataclasses import dataclass\n\n@topic\n@dataclass\nclass OrderPlaced:\n    id: str\n",
            ),
            (
                "server/models/OrderPlaced.java",
                "@Topic\npublic record OrderPlaced(String id) {}\n",
            ),
            (
                "server/models/OrderPlaced.kt",
                "@Topic\ndata class OrderPlaced(val id: String)\n",
            ),
            (
                "server/models/OrderPlaced.cs",
                "[Topic]\npublic record OrderPlaced(string Id);\n",
            ),
            (
                "server/models/OrderPlaced.rs",
                "#[Topic]\npub struct OrderPlaced {\n    pub id: String,\n}\n",
            ),
            (
                "server/models/OrderPlaced.go",
                "package main\n\n// @Topic\ntype OrderPlaced struct {\n\tID string `json:\"id\"`\n}\n",
            ),
            (
                "server/models/OrderPlaced.dart",
                "@Topic\nclass OrderPlaced {\n  final String id;\n  OrderPlaced(this.id);\n}\n",
            ),
            (
                "server/models/OrderPlaced.php",
                "<?php\n\n#[Topic]\nclass OrderPlaced\n{\n    public string $id;\n}\n",
            ),
            (
                "server/models/OrderPlaced.ts",
                "/** @Topic */\nexport interface OrderPlaced {\n  id: string\n}\n",
            ),
            (
                "server/models/OrderPlaced.rb",
                "#[Topic]\nOrderPlaced = Data.define(:id)\n",
            ),
            (
                "server/models/OrderPlaced.fs",
                "[<Topic>]\ntype OrderPlaced = { Id: string }\n",
            ),
            (
                "client/models/OrderPlaced.fs",
                "[<Topic>]\ntype OrderPlaced = { Id: string }\n",
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
        let source = "@Topic\nclass OrderPlaced:\n    id: str\n";
        let models = declared(&["OrderPlaced"]);
        assert!(Topics::check(Path::new("server/models/OrderPlaced.py"), source, &models).is_ok());
        let outside = Topics::check(Path::new("server/services/Orders.py"), source, &models)
            .expect_err("outside models");
        assert!(format!("{outside:?}").contains("models folder"));
        let alone = Topics::check(Path::new("server/models/Nothing.py"), "@Topic\n", &[])
            .expect_err("no model");
        assert!(format!("{alone:?}").contains("above a model declaration"));
        let lower = "@Topic\nclass orderPlaced:\n    id: str\n";
        assert!(
            Topics::check(
                Path::new("server/models/x.py"),
                lower,
                &declared(&["orderPlaced"])
            )
            .is_err()
        );
    }
}
