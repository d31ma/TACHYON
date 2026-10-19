use super::fsharp_service_needs::FsharpServiceNeeds;
use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::fsharp_source::FsharpSource;
use crate::project::NativeCompanion;
use dotnet_page_program::DotnetPageProgram;
use kotlin_page_program::KotlinPageProgram;
use rust_page_program::RustPageProgram;
use std::collections::BTreeSet;
use swift_page_program::SwiftPageProgram;

mod dotnet_page_program;
mod kotlin_page_program;
mod rust_page_program;
mod shared_page_program;
mod swift_page_program;

pub(super) type PageServices<'page, 'imports> = (
    &'page [&'page NativeCompanionInput],
    &'page FsharpServiceNeeds,
    &'imports mut BTreeSet<String>,
);

#[derive(Default)]
pub(super) struct PageProgram {
    pub(super) namespaces: String,
    pub(super) fsharp: String,
    pub(super) runtimes: String,
    pub(super) services: Vec<(String, String)>,
}

impl PageProgram {
    pub(super) fn lowered(
        language: NativeCompanion,
        services: PageServices<'_, '_>,
    ) -> Result<Self, Failure> {
        Self::language_program(language)(services)
    }

    pub(super) fn each_service(
        language: NativeCompanion,
        (selected, needs, imports): PageServices<'_, '_>,
    ) -> Result<Self, Failure> {
        selected
            .iter()
            .enumerate()
            .try_fold(Self::default(), |program, (index, item)| {
                program.with_service(language, (index, item), needs, imports)
            })
    }

    fn language_program(
        language: NativeCompanion,
    ) -> fn(PageServices<'_, '_>) -> Result<Self, Failure> {
        match language {
            NativeCompanion::Swift => SwiftPageProgram::lowered,
            NativeCompanion::Kotlin => KotlinPageProgram::lowered,
            NativeCompanion::CSharp => DotnetPageProgram::lowered,
            NativeCompanion::Rust => RustPageProgram::lowered,
        }
    }

    fn with_service(
        mut self,
        language: NativeCompanion,
        (index, item): (usize, &NativeCompanionInput),
        needs: &FsharpServiceNeeds,
        imports: &mut BTreeSet<String>,
    ) -> Result<Self, Failure> {
        let body = super::registry::service_body(language, item, needs, imports)?;
        let namespace = format!("TachyonService{index}");
        let (body, fsharp_body) = FsharpSource::web_split(&body);
        let (fsharp_body, runtime) = FsharpSource::runtime_split(fsharp_body);
        self.namespaces
            .push_str(&Self::namespace_source(language, &namespace, body));
        self.fsharp
            .push_str(&FsharpSource::web_namespace(&namespace, fsharp_body));
        self.runtimes
            .push_str(&FsharpSource::runtime_namespace(&namespace, runtime));
        self.services.push((item.service.clone(), namespace));
        Ok(self)
    }

    fn namespace_source(language: NativeCompanion, namespace: &str, source: &str) -> String {
        match language {
            NativeCompanion::Swift => format!("private enum {namespace} {{\n{source}\n}}\n"),
            NativeCompanion::Kotlin => format!("private object {namespace} {{\n{source}\n}}\n"),
            NativeCompanion::CSharp => format!("namespace {namespace} {{\n{source}\n}}\n"),
            NativeCompanion::Rust => {
                format!(
                    "#[allow(non_snake_case)]\nmod {namespace} {{\nuse super::*;\n{source}\n}}\n"
                )
            }
        }
    }
}
