use super::super::super::registry::{self, CompiledService, NativeCompanionInput};
use super::super::PageProgram;
use super::SwiftPageProgram;
use crate::Failure;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

pub(super) struct SwiftPage {
    source: String,
    members: Vec<String>,
}

impl SwiftPage {
    pub(super) fn lowered(
        first: &NativeCompanionInput,
        (identity, compiled): (&Path, Vec<CompiledService>),
        unlowered: String,
    ) -> Result<Self, Failure> {
        let source = Self::lowered_source(first, identity, &compiled, unlowered)?;
        Ok(Self {
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
            namespaces: registry::body_without_imports(
                SwiftPageProgram::LANGUAGE,
                &self.source,
                imports,
            ),
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
            let _ = writeln!(
                program.namespaces,
                "private enum {namespace} {{\n{members}\n}}"
            );
            program.services.push((item.service.clone(), namespace));
        }
    }

    fn lowered_source(
        first: &NativeCompanionInput,
        identity: &Path,
        compiled: &[CompiledService],
        unlowered: String,
    ) -> Result<String, Failure> {
        let models = registry::client_models(first, SwiftPageProgram::LANGUAGE)?;
        let lowered = registry::task_lowered(first, &models, identity, &unlowered)?;
        let language = super::super::super::events::companion_language(SwiftPageProgram::LANGUAGE);
        Ok(compiled
            .iter()
            .fold(lowered.unwrap_or(unlowered), |source, service| {
                crate::companion::with_required_constructor(
                    language,
                    &source,
                    &service.portable,
                    service.needs_all,
                )
            }))
    }
}
