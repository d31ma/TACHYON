use super::PageProgram;
use super::shared_page::SharedPage;
use crate::Failure;
use crate::annotation_modules::PageParts;
use crate::native::registry::{self, NativeCompanionInput};
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct CsharpPage<'page> {
    page: SharedPage<'page>,
}

impl<'page> CsharpPage<'page> {
    const NAMESPACE: &'static str = "TachyonServiceCsharp";
    const LANGUAGE: NativeCompanion = NativeCompanion::CSharp;

    pub(super) fn shares(item: &NativeCompanionInput) -> bool {
        [
            item.language == Self::LANGUAGE,
            !crate::fsharp_source::FsharpSource::is_fsharp(&item.source),
        ] == [true, true]
    }

    pub(super) fn add(
        &mut self,
        (index, item): (usize, &'page NativeCompanionInput),
    ) -> Result<(), Failure> {
        let compiled = registry::page_service(Self::LANGUAGE, item)?;
        let entry = Self::relative(item).to_string_lossy().replace('\\', "/");
        PageParts::split(&compiled.unlowered, &entry)
            .iter()
            .try_for_each(|(path, text)| self.page.add_part(path, text))?;
        self.page.add_service(index, item, compiled.members);
        Ok(())
    }

    pub(super) fn written(self, program: PageProgram) -> Result<PageProgram, Failure> {
        self.page
            .with_first(program, |first, program| self.shared(first, program))
    }

    fn shared(
        &self,
        first: &NativeCompanionInput,
        program: PageProgram,
    ) -> Result<PageProgram, Failure> {
        let identity = registry::service_identity(Self::LANGUAGE, &first.class);
        let ordered: Vec<String> = self
            .page
            .parts()
            .into_iter()
            .map(|(_, text)| text.to_owned())
            .collect();
        let composed = PageParts::program(&Self::relative(first), &ordered)?;
        let usings = PageParts::usings(&composed);
        let models = registry::client_models(first, Self::LANGUAGE)?;
        let lowered = registry::task_lowered(first, &models, &identity, &composed)?;
        let lowered = lowered.unwrap_or(composed);
        Ok(self
            .page
            .written(program, Self::NAMESPACE, (&lowered, &usings)))
    }

    fn relative(item: &NativeCompanionInput) -> PathBuf {
        item.source
            .strip_prefix(&item.snapshot_root)
            .unwrap_or(Path::new(&item.class))
            .to_path_buf()
    }
}
