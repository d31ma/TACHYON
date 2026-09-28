#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;
use super::declared_name::DeclaredName;
use super::declared_names::DeclaredNames;
use super::design_problem::DesignProblem;
use super::design_rule::DesignRule;
use super::name_parts::NameParts;
use super::project_words::ProjectWords;
use super::word_check::WordCheck;
use std::collections::BTreeSet;

pub(super) struct NameRule;

impl NameRule {
    const CLIENT_PREFIX: &'static str = "client/";
    const WORD_LISTS: [&'static str; 2] = [
        "`names.words` in server/application.json",
        "`tac.names.words` in client/manifest.json",
    ];

    pub(super) fn problems(classes: &[ClassFile], words: &ProjectWords) -> Vec<DesignProblem> {
        classes
            .iter()
            .flat_map(|class| Self::class_problems(class, words.for_path(class.path)))
            .collect()
    }

    fn class_problems(class: &ClassFile, project: &[String]) -> Vec<DesignProblem> {
        let mut seen = BTreeSet::new();
        DeclaredNames::read(class)
            .into_iter()
            .filter(|name| seen.insert(class.words.text(name.at)))
            .filter_map(|name| Self::problem(class, &name, project))
            .collect()
    }

    fn problem(
        class: &ClassFile,
        name: &DeclaredName,
        project: &[String],
    ) -> Option<DesignProblem> {
        let text = class.words.text(name.at);
        let (part, check) = NameParts::split(text)
            .into_iter()
            .map(|part| {
                let check = WordCheck::of(&part, project);
                (part, check)
            })
            .find(|(_, check)| *check != WordCheck::Plain)?;
        let message = format!(
            "The {} name `{text}` in {} is not plain English: {}.",
            name.kind,
            class.name,
            Self::reason(&part, check)
        );
        let help = Self::help(class, text, &part, check);
        Some(class.problem(DesignRule::Names, name.at, message, help))
    }

    fn reason(part: &str, check: WordCheck) -> String {
        let WordCheck::Clipped(full) = check else {
            return format!("`{part}` is not an English word");
        };
        format!("`{part}` is short for `{full}`")
    }

    fn help(class: &ClassFile, text: &str, part: &str, check: WordCheck) -> String {
        let WordCheck::Clipped(full) = check else {
            let client = class.path.starts_with(Self::CLIENT_PREFIX);
            return format!(
                "Write whole English words. If `{part}` is a name from your domain, such as a brand or a product, add it to {}.",
                Self::WORD_LISTS[usize::from(client)]
            );
        };
        format!(
            "Rename it to `{}`: write whole English words.",
            Self::renamed(text, part, full)
        )
    }

    fn renamed(text: &str, part: &str, full: &str) -> String {
        let at = text.to_ascii_lowercase().find(part).unwrap_or(0);
        let original = &text[at..at + part.len()];
        let capital = original.starts_with(|first: char| first.is_ascii_uppercase());
        let upper = original.chars().all(|letter| letter.is_ascii_uppercase()) & (part.len() > 1);
        let replaced = [
            full.to_owned(),
            Self::capitalized(full),
            full.to_ascii_uppercase(),
        ][usize::from(capital) + usize::from(upper)]
        .clone();
        format!("{}{replaced}{}", &text[..at], &text[at + part.len()..])
    }

    fn capitalized(word: &str) -> String {
        let mut letters = word.chars();
        letters
            .next()
            .map(|first| first.to_ascii_uppercase().to_string() + letters.as_str())
            .unwrap_or_default()
    }
}
