use super::super::registry::{self, CompiledService, NativeCompanionInput};
use super::{PageProgram, PageServices};
use crate::Failure;
use crate::project::NativeCompanion;
use kept_declarations::KeptDeclarations;
use page_declarations::PageDeclarations;
use shared_page::SharedPage;
use std::path::Path;

mod kept_declarations;
mod page_declarations;
mod shared_page;

pub(super) struct SharedPageProgram {
    language: NativeCompanion,
}

impl SharedPageProgram {
    pub(super) fn lowered(
        language: NativeCompanion,
        services: PageServices<'_, '_>,
    ) -> Result<PageProgram, Failure> {
        let (selected, needs, imports) = services;
        let program = Self { language };
        let page = program.page(selected)?;
        program.program(page, (selected, needs, imports))
    }

    fn program(
        &self,
        page: Option<SharedPage>,
        (selected, needs, imports): PageServices<'_, '_>,
    ) -> Result<PageProgram, Failure> {
        match page {
            Some(page) => Ok(page.program(selected, imports)),
            None => PageProgram::each_service(self.language, (selected, needs, imports)),
        }
    }

    fn page(&self, selected: &[&NativeCompanionInput]) -> Result<Option<SharedPage>, Failure> {
        let one_kind = selected
            .windows(2)
            .all(|pair| pair[0].web_assembly == pair[1].web_assembly);
        one_kind
            .then(|| self.shared_page(selected))
            .transpose()
            .map(Option::flatten)
    }

    fn shared_page(
        &self,
        selected: &[&NativeCompanionInput],
    ) -> Result<Option<SharedPage>, Failure> {
        let compiled = self.compiled(selected)?;
        let first = selected[0];
        let identity = registry::service_identity(self.language, &first.class);
        let unlowered = Self::page_source(&identity, &compiled)?;
        unlowered
            .map(|text| SharedPage::lowered(self.language, first, (&identity, compiled), text))
            .transpose()
    }

    fn compiled(
        &self,
        selected: &[&NativeCompanionInput],
    ) -> Result<Vec<CompiledService>, Failure> {
        selected
            .iter()
            .map(|item| registry::page_service(self.language, item))
            .collect()
    }

    fn page_source(
        identity: &Path,
        compiled: &[CompiledService],
    ) -> Result<Option<String>, Failure> {
        let declarations = Self::all_declarations(identity, compiled)?;
        let kept = declarations
            .iter()
            .try_fold(KeptDeclarations::default(), |kept, declaration| {
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
            .map(|service| PageDeclarations::of(identity, &service.unlowered))
            .collect::<Result<Vec<_>, Failure>>()?;
        Ok(each.concat())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{CompiledService, PageDeclarations, SharedPageProgram};
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

    fn page(identity: &str, services: &[&str]) -> Option<String> {
        let compiled = services
            .iter()
            .map(|text| service(text))
            .collect::<Vec<_>>();
        SharedPageProgram::page_source(Path::new(identity), &compiled).expect("page source")
    }

    #[test]
    fn a_declaration_that_two_services_share_is_kept_once() {
        let setup = "final class NotesSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters { BehaviorAdapters() }\n}\n";
        let writer = format!("final class WriterService {{\n    var count = 0\n}}\n\n{setup}");
        let reader = format!("final class ReaderService {{\n    var count = 0\n}}\n\n{setup}");
        let source = page("client/services/WriterService.swift", &[&writer, &reader])
            .expect("one page program");
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
        assert_eq!(
            page("client/services/WriterService.swift", &[writer, reader]),
            None
        );
    }

    #[test]
    fn braces_inside_strings_do_not_split_a_declaration() {
        let source = "final class WriterService {\n    let text = \"}\"\n    func run() {}\n}\nstruct Note {}\n";
        let declarations =
            PageDeclarations::of(Path::new("client/services/WriterService.swift"), source)
                .expect("declarations");
        assert_eq!(declarations.len(), 2, "{declarations:?}");
        assert!(declarations[0].contains("func run()"), "{declarations:?}");
    }

    #[test]
    fn a_kotlin_store_without_braces_stays_with_the_declaration_after_it() {
        let store = "@Store(NoteEntity::class)\ninterface NoteStore : Repository<NoteEntity, String>\n\nclass NoteSetup {\n    val text = \"${1}\"\n}\n";
        let writer = format!("class WriterService {{\n    var count = 0\n}}\n\n{store}");
        let reader = format!("class ReaderService {{\n    var count = 0\n}}\n\n{store}");
        let source = page("client/services/WriterService.kt", &[&writer, &reader])
            .expect("one page program");
        assert_eq!(source.matches("interface NoteStore").count(), 1, "{source}");
        assert_eq!(source.matches("class NoteSetup").count(), 1, "{source}");
    }
}
