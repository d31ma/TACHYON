use super::{PageProgram, PageServices};
use crate::Failure;
use crate::native::registry::{self, NativeCompanionInput};
use crate::project::NativeCompanion;
use rust_page_owner::RustPageOwner;
use std::path::{Path, PathBuf};

mod rust_page_owner;
#[cfg(test)]
mod tests;

pub(super) struct RustPageProgram {
    units: Vec<(String, String)>,
    owners: Vec<RustPageOwner>,
}

impl RustPageProgram {
    const UNIT: &'static str = "//__TACHYON_PAGE_UNIT__ ";
    const PAGE: &'static str = "TachyonPage";

    pub(super) fn lowered((selected, _, _): PageServices<'_, '_>) -> Result<PageProgram, Failure> {
        let mut page = Self {
            units: Vec::new(),
            owners: Vec::new(),
        };
        selected.iter().try_for_each(|item| page.add(item))?;
        page.program(selected[0])
    }

    fn add(&mut self, item: &NativeCompanionInput) -> Result<(), Failure> {
        let captured = registry::read_captured_source(&item.source)?;
        let captured = crate::without_bom(&captured);
        let identity = registry::service_identity(NativeCompanion::Rust, &item.class);
        let relative = item
            .source
            .strip_prefix(&item.snapshot_root)
            .unwrap_or(&identity)
            .to_path_buf();
        let roles = crate::behavior::ServiceRoles::read(&relative, captured)?;
        let units = crate::annotation_modules::Module::page_units(
            &item.snapshot_root,
            &relative,
            &roles.source,
        )?;
        let marked = Self::marked(&units);
        let models = registry::client_models(item, NativeCompanion::Rust)?;
        let mut events = crate::events::extract(&identity, &marked, "tac")?;
        events
            .as_mut()
            .map(|module| crate::events::describe(module, &identity, &item.snapshot_root, "client"))
            .transpose()?;
        crate::route_types::Converters::require_rust_program(
            NativeCompanion::Rust,
            &item.snapshot_root,
            (&relative, &models.assembled(&relative, &marked)?),
            events.as_ref(),
        )?;
        let prepared = crate::native::events::prepare(
            NativeCompanion::Rust,
            &marked,
            events.as_ref(),
            &identity,
        )?;
        self.split(&prepared.source);
        self.owners.push(RustPageOwner {
            service: item.service.clone(),
            identity,
            relative,
            portable: item.source.to_string_lossy().into_owned(),
            excluded: prepared.excluded,
            entries: prepared.entries,
            required: roles.required,
        });
        Ok(())
    }

    fn marked(units: &[(PathBuf, String)]) -> String {
        units
            .iter()
            .map(|(path, text)| format!("{}{}\n{text}\n", Self::UNIT, path.to_string_lossy()))
            .collect::<Vec<_>>()
            .concat()
    }

    fn split(&mut self, rewritten: &str) {
        let mut units = rewritten
            .split(Self::UNIT)
            .skip(1)
            .map(|unit| unit.split_once('\n').unwrap_or((unit, "")));
        let entry = units
            .next()
            .map(|(path, text)| (path.to_owned(), text.to_owned()));
        self.units.extend(entry);
        for (path, text) in units {
            let known = self.units.iter().any(|(seen, _)| seen == path);
            self.units
                .extend((!known).then(|| (path.to_owned(), text.to_owned())));
        }
    }

    fn program(&self, first: &NativeCompanionInput) -> Result<PageProgram, Failure> {
        let owner = &self.owners[0];
        let mut imports = crate::native::rust_imports::RustImports::default();
        let bodies = self
            .units
            .iter()
            .map(|(path, text)| imports.take(Path::new(path), text))
            .collect::<Result<Vec<_>, Failure>>()?;
        let page = format!("{}\n{}", imports.source(), bodies.join("\n\n"));
        let models = registry::client_models(first, NativeCompanion::Rust)?;
        let lowered = models
            .lowered(&owner.identity, &page, |text| {
                crate::behavior::lower(&owner.identity, text, "tac")
            })?
            .unwrap_or(page);
        let services = self
            .owners
            .iter()
            .enumerate()
            .map(|(index, owner)| {
                Self::service_module(index, owner, &lowered, &first.snapshot_root)
            })
            .collect::<Result<String, Failure>>()?;
        Ok(PageProgram {
            namespaces: format!(
                "#[allow(non_snake_case)]\nmod {} {{\nuse super::*;\n{lowered}\n{services}}}\n",
                Self::PAGE
            ),
            services: self
                .owners
                .iter()
                .enumerate()
                .map(|(index, owner)| {
                    (
                        owner.service.clone(),
                        format!("{}::TachyonService{index}", Self::PAGE),
                    )
                })
                .collect(),
            ..PageProgram::default()
        })
    }

    fn service_module(
        index: usize,
        owner: &RustPageOwner,
        lowered: &str,
        root: &Path,
    ) -> Result<String, Failure> {
        crate::route_types::Converters::require_rust_selected_owner(
            NativeCompanion::Rust,
            root,
            (&owner.relative, lowered),
            &owner.excluded,
        )?;
        let table = crate::companion::owner_table(
            crate::native::events::companion_language(NativeCompanion::Rust).for_web(true),
            lowered,
            &owner.portable,
            &owner.excluded,
            &owner.entries,
            !owner.required.is_empty(),
        )?;
        let required = owner
            .required
            .iter()
            .map(|name| format!("{name:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!(
            "#[allow(non_snake_case)]\npub(super) mod TachyonService{index} {{\nuse super::*;\n{table}pub const TAC_REQUIRED: &[&str] = &[{required}];\n}}\n"
        ))
    }
}
