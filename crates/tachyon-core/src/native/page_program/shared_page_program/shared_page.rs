use super::super::super::registry::{self, CompiledService, NativeCompanionInput};
use super::super::PageProgram;
use crate::Failure;
use crate::project::NativeCompanion;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct SharedPage {
    language: NativeCompanion,
    source: String,
    members: Vec<String>,
}

impl SharedPage {
    pub(super) fn lowered(
        language: NativeCompanion,
        first: &NativeCompanionInput,
        (identity, compiled): (&Path, Vec<CompiledService>),
        unlowered: String,
    ) -> Result<Self, Failure> {
        let source = Self::lowered_source(language, first, (identity, &compiled), unlowered)?;
        Ok(Self {
            language,
            source,
            members: Self::members(compiled),
        })
    }

    pub(super) fn program(
        self,
        selected: &[&NativeCompanionInput],
        imports: &mut BTreeSet<String>,
    ) -> PageProgram {
        let mut program = PageProgram {
            namespaces: registry::body_without_imports(self.language, &self.source, imports),
            ..PageProgram::default()
        };
        self.add_namespaces(&mut program, selected);
        program
    }

    fn members(compiled: Vec<CompiledService>) -> Vec<String> {
        compiled
            .into_iter()
            .map(|service| service.members)
            .collect()
    }

    fn add_namespaces(&self, program: &mut PageProgram, selected: &[&NativeCompanionInput]) {
        for (index, (item, members)) in selected.iter().zip(&self.members).enumerate() {
            let namespace = format!("TachyonService{index}");
            program.namespaces.push_str(&PageProgram::namespace_source(
                self.language,
                &namespace,
                members,
            ));
            program.services.push((item.service.clone(), namespace));
        }
    }

    fn lowered_source(
        language: NativeCompanion,
        first: &NativeCompanionInput,
        (identity, compiled): (&Path, &[CompiledService]),
        unlowered: String,
    ) -> Result<String, Failure> {
        let models = registry::client_models(first, language)?;
        let lowered = registry::task_lowered(first, &models, identity, &unlowered)?;
        let companion = super::super::super::events::companion_language(language);
        Ok(compiled
            .iter()
            .fold(lowered.unwrap_or(unlowered), |source, service| {
                crate::companion::with_required_constructor(
                    companion,
                    &source,
                    &service.portable,
                    service.needs_all,
                )
            }))
    }
}
