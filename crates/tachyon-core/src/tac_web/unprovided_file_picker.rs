use super::WebLanguage;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::{Path, PathBuf};

pub(super) struct UnprovidedFilePicker;

impl UnprovidedFilePicker {
    const NAME: &'static str = "FilePicker";
    const CODE: u16 = 1407;
    const REFUSED: [(WebLanguage, &'static str, &'static str); 2] = [
        (WebLanguage::Java, "java", "Java"),
        (WebLanguage::Go, "go", "Go"),
    ];
    const HELP: &'static str = "Open the file picker from a JavaScript, TypeScript, Rust, Swift, C# or Kotlin service (ADR 0089), and pass what it reads to this service.";

    pub(super) fn refuse(
        snapshot_root: &Path,
        language: WebLanguage,
        chosen: &[(String, String, PathBuf)],
    ) -> Result<(), Failure> {
        Self::REFUSED
            .iter()
            .find(|(refused, _, _)| *refused == language)
            .map_or(Ok(()), |(_, extension, label)| {
                chosen.iter().try_for_each(|(_, class, path)| {
                    Self::refuse_service(snapshot_root, (extension, label), class, path)
                })
            })
    }

    fn refuse_service(
        snapshot_root: &Path,
        (extension, label): (&str, &str),
        class: &str,
        path: &Path,
    ) -> Result<(), Failure> {
        let Ok(source) = std::fs::read_to_string(path) else {
            return Ok(());
        };
        let code = crate::lexical::code_mask(extension, &source).code;
        let relative = path
            .strip_prefix(snapshot_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        source
            .match_indices(Self::NAME)
            .map(|(at, _)| at)
            .find(|at| Self::is_code_word(&source, &code, *at))
            .map_or(Ok(()), |at| {
                Err(Failure::one(diagnostic(
                    Self::CODE,
                    format!("{class} needs a FilePicker, but {label} services have no FilePicker provider, so the page could never fill that field."),
                    Some(String::from(Self::HELP)),
                    source_span(&relative, at, at + Self::NAME.len()),
                )))
            })
    }

    fn is_code_word(source: &str, code: &[bool], at: usize) -> bool {
        let before = source[..at].chars().next_back();
        let after = source[at + Self::NAME.len()..].chars().next();
        [
            code.get(at).copied().unwrap_or(false),
            !Self::continues_name(before),
            !Self::continues_name(after),
        ] == [true; 3]
    }

    fn continues_name(character: Option<char>) -> bool {
        character.is_some_and(|character| {
            [character.is_alphanumeric(), character == '_'].contains(&true)
        })
    }
}
