use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::project::NativeCompanion;
use crate::route_types::ModelTable;
use std::path::Path;

pub(super) struct CsharpTaskInventory;

impl CsharpTaskInventory {
    pub(super) fn lower(
        item: &NativeCompanionInput,
        path: &Path,
        source: &str,
    ) -> Result<Option<String>, Failure> {
        Self::selected_lower(item, path, source)
    }

    pub(super) fn entries(
        item: &NativeCompanionInput,
        source: &str,
        origin: (&Path, &[String]),
        extra: &str,
    ) -> Result<String, Failure> {
        Self::selected_entries(item, source, origin, extra)
    }

    fn selected(item: &NativeCompanionInput) -> bool {
        item.web_assembly && item.language == NativeCompanion::CSharp
    }

    fn selected_lower(
        item: &NativeCompanionInput,
        path: &Path,
        source: &str,
    ) -> Result<Option<String>, Failure> {
        let lower = Self::lowerer(Self::selected(item));
        lower(item, path, source)
    }

    fn selected_entries(
        item: &NativeCompanionInput,
        source: &str,
        origin: (&Path, &[String]),
        extra: &str,
    ) -> Result<String, Failure> {
        let entries = Self::selected(item)
            .then(|| Self::model_entries(item, source, origin))
            .transpose()?;
        Ok(format!("{}{extra}", entries.as_deref().unwrap_or("")))
    }

    fn model_entries(
        item: &NativeCompanionInput,
        source: &str,
        origin: (&Path, &[String]),
    ) -> Result<String, Failure> {
        crate::companion::CsharpTaskMembers::entries(
            source,
            &origin.0.to_string_lossy(),
            &Self::models(item, origin.0)?,
            origin.1,
        )
    }

    fn lowerer(
        selected: bool,
    ) -> fn(&NativeCompanionInput, &Path, &str) -> Result<Option<String>, Failure> {
        if selected {
            Self::model_lower
        } else {
            Self::plain_lower
        }
    }

    fn model_lower(
        item: &NativeCompanionInput,
        path: &Path,
        source: &str,
    ) -> Result<Option<String>, Failure> {
        crate::behavior::CsharpTaskModels::lower(path, source, &Self::models(item, path)?)
    }

    fn plain_lower(
        _item: &NativeCompanionInput,
        path: &Path,
        source: &str,
    ) -> Result<Option<String>, Failure> {
        crate::behavior::lower(path, source, "tac")
    }

    fn models(item: &NativeCompanionInput, path: &Path) -> Result<ModelTable, Failure> {
        ModelTable::read_realm(&item.snapshot_root, "client", "cs").map_err(|message| {
            Failure::one(crate::failure::diagnostic(
                1310,
                message,
                None,
                crate::failure::source_span(&path.to_string_lossy(), 0, 1),
            ))
        })
    }
}
