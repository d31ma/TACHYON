use super::PageProgram;
use crate::Failure;
use crate::native::registry::NativeCompanionInput;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct SharedPage<'page> {
    parts: Vec<String>,
    texts: BTreeMap<String, String>,
    services: Vec<(usize, &'page NativeCompanionInput, String)>,
}

impl<'page> SharedPage<'page> {
    const FAILURE_CODE: u16 = 1605;
    const CONFLICT: &'static str = "One authored path produced different page declarations for two services on this page, so the page cannot share one copy of it.";

    pub(super) fn add_service(
        &mut self,
        index: usize,
        item: &'page NativeCompanionInput,
        members: String,
    ) {
        self.services.push((index, item, members));
    }

    pub(super) fn add_part(&mut self, path: &str, text: &str) -> Result<(), Failure> {
        let same = self.texts.get(path).is_none_or(|previous| previous == text);
        let fresh = !self.texts.contains_key(path);
        self.parts.extend(fresh.then(|| path.to_owned()));
        self.texts
            .entry(path.to_owned())
            .or_insert_with(|| text.to_owned());
        same.then_some(())
            .ok_or_else(|| Self::conflicting_source(path))
    }

    pub(super) fn with_first(
        &self,
        program: PageProgram,
        mut build: impl FnMut(&NativeCompanionInput, PageProgram) -> Result<PageProgram, Failure>,
    ) -> Result<PageProgram, Failure> {
        self.services
            .first()
            .into_iter()
            .try_fold(program, |program, (_, first, _)| build(first, program))
    }

    pub(super) fn parts(&self) -> Vec<(&str, &str)> {
        self.parts
            .iter()
            .map(|path| {
                (
                    path.as_str(),
                    self.texts.get(path).map_or("", String::as_str),
                )
            })
            .collect()
    }

    pub(super) fn written(
        &self,
        mut program: PageProgram,
        namespace: &str,
        (page, usings): (&str, &str),
    ) -> PageProgram {
        let glue = self
            .services
            .iter()
            .map(|(index, _, members)| {
                format!(
                    "namespace TachyonService{index} {{\n{usings}using {namespace};\n{members}\n}}\n"
                )
            })
            .collect::<Vec<_>>()
            .concat();
        program.services.extend(
            self.services
                .iter()
                .map(|(index, item, _)| (item.service.clone(), format!("TachyonService{index}"))),
        );
        program.namespaces = format!(
            "{}namespace {namespace} {{\n{page}\n}}\n{glue}",
            program.namespaces
        );
        program
    }

    fn conflicting_source(path: &str) -> Failure {
        Failure::one(crate::failure::diagnostic(
            Self::FAILURE_CODE,
            Self::CONFLICT,
            None,
            crate::failure::source_span(path, 0, 1),
        ))
    }
}
