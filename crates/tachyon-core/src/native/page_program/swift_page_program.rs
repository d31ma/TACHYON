use super::super::registry::{self, CompiledService, NativeCompanionInput};
use super::{PageProgram, PageServices};
use crate::Failure;
use crate::project::NativeCompanion;
use std::path::Path;
use swift_declarations::SwiftDeclarations;
use swift_kept_declarations::SwiftKeptDeclarations;
use swift_page::SwiftPage;

mod swift_declarations;
mod swift_kept_declarations;
mod swift_page;

pub(super) struct SwiftPageProgram;

impl SwiftPageProgram {
    const LANGUAGE: NativeCompanion = NativeCompanion::Swift;

    pub(super) fn lowered(services: PageServices<'_, '_>) -> Result<PageProgram, Failure> {
        let (selected, needs, imports) = services;
        let page = Self::page(selected)?;
        Self::program(page, (selected, needs, imports))
    }

    fn program(
        page: Option<SwiftPage>,
        (selected, needs, imports): PageServices<'_, '_>,
    ) -> Result<PageProgram, Failure> {
        match page {
            Some(page) => Ok(page.program(selected, imports)),
            None => PageProgram::each_service(Self::LANGUAGE, (selected, needs, imports)),
        }
    }

    fn page(selected: &[&NativeCompanionInput]) -> Result<Option<SwiftPage>, Failure> {
        let in_browser = selected.iter().all(|item| item.web_assembly);
        in_browser
            .then(|| Self::browser_page(selected))
            .transpose()
            .map(Option::flatten)
    }

    fn browser_page(selected: &[&NativeCompanionInput]) -> Result<Option<SwiftPage>, Failure> {
        let compiled = Self::compiled(selected)?;
        let first = selected[0];
        let identity = registry::service_identity(Self::LANGUAGE, &first.class);
        let unlowered = Self::page_source(&identity, &compiled)?;
        unlowered
            .map(|text| SwiftPage::lowered(first, (&identity, compiled), text))
            .transpose()
    }

    fn compiled(selected: &[&NativeCompanionInput]) -> Result<Vec<CompiledService>, Failure> {
        selected
            .iter()
            .map(|item| registry::page_service(Self::LANGUAGE, item))
            .collect()
    }

    fn page_source(
        identity: &Path,
        compiled: &[CompiledService],
    ) -> Result<Option<String>, Failure> {
        let declarations = Self::all_declarations(identity, compiled)?;
        let kept = declarations
            .iter()
            .try_fold(SwiftKeptDeclarations::default(), |kept, declaration| {
                kept.with(identity, declaration)
            })?;
        Ok(kept.source())
    }

    fn all_declarations(
        identity: &Path,
        compiled: &[CompiledService],
    ) -> Result<Vec<String>, Failure> {
        let each = compiled
            .iter()
            .map(|service| SwiftDeclarations::of(identity, &service.unlowered))
            .collect::<Result<Vec<_>, Failure>>()?;
        Ok(each.concat())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{CompiledService, SwiftDeclarations, SwiftPageProgram};
    use std::path::Path;

    fn service(unlowered: &str) -> CompiledService {
        CompiledService {
            unlowered: unlowered.to_owned(),
            source: String::new(),
            members: String::new(),
            portable: String::new(),
            needs_all: false,
        }
    }

    fn page(services: &[&str]) -> Option<String> {
        let compiled = services
            .iter()
            .map(|text| service(text))
            .collect::<Vec<_>>();
        SwiftPageProgram::page_source(Path::new("client/services/WriterService.swift"), &compiled)
            .expect("page source")
    }

    #[test]
    fn a_declaration_that_two_services_share_is_kept_once() {
        let setup = "final class NotesSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters { BehaviorAdapters() }\n}\n";
        let writer = format!("final class WriterService {{\n    var count = 0\n}}\n\n{setup}");
        let reader = format!("final class ReaderService {{\n    var count = 0\n}}\n\n{setup}");
        let source = page(&[&writer, &reader]).expect("one page program");
        assert_eq!(
            source.matches("final class NotesSetup").count(),
            1,
            "{source}"
        );
        assert!(source.contains("final class WriterService"), "{source}");
        assert!(source.contains("final class ReaderService"), "{source}");
    }

    #[test]
    fn two_different_declarations_with_one_name_keep_one_program_for_each_service() {
        let writer =
            "final class WriterService {}\n\nfinal class NotesSetup {\n    let size = 1\n}\n";
        let reader =
            "final class ReaderService {}\n\nfinal class NotesSetup {\n    let size = 2\n}\n";
        assert_eq!(page(&[writer, reader]), None);
    }

    #[test]
    fn braces_inside_strings_do_not_split_a_declaration() {
        let source = "final class WriterService {\n    let text = \"}\"\n    func run() {}\n}\nstruct Note {}\n";
        let declarations =
            SwiftDeclarations::of(Path::new("client/services/WriterService.swift"), source)
                .expect("declarations");
        assert_eq!(declarations.len(), 2, "{declarations:?}");
        assert!(declarations[0].contains("func run()"), "{declarations:?}");
    }
}
