use super::native_menu_action::NativeMenuAction;
use super::native_menu_config::NativeMenuConfig;
use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::project::NativeCompanion;
use std::path::Path;

pub(super) struct NativeMenuActionCheck;

impl NativeMenuActionCheck {
    const CODE: u16 = 1370;
    const MAX_DEPTH: usize = 16;
    const HELP: &'static str = "A menu action calls a method that takes only &self on a client Rust Service marked #[Lifetime({\"value\":\"application\"})]. Its optional event names a Rust model in client/models.";

    pub(super) fn validate(
        config: &NativeMenuConfig,
        companions: &[NativeCompanionInput],
    ) -> Result<(), Failure> {
        config
            .items()
            .filter_map(|item| {
                item.action
                    .as_ref()
                    .map(|action| (item.id.as_str(), action))
            })
            .try_for_each(|(id, action)| Self::validate_action(id, action, companions))
    }

    fn validate_action(
        id: &str,
        action: &NativeMenuAction,
        companions: &[NativeCompanionInput],
    ) -> Result<(), Failure> {
        let provider = Self::provider(id, action, companions)?;
        let source = std::fs::read_to_string(&provider.source).unwrap_or_default();
        Self::require_method(id, action, &source)?;
        action.event.as_deref().map_or(Ok(()), |event| {
            Self::require_model(id, event, &provider.snapshot_root)
        })
    }

    fn provider<'a>(
        id: &str,
        action: &NativeMenuAction,
        companions: &'a [NativeCompanionInput],
    ) -> Result<&'a NativeCompanionInput, Failure> {
        companions
            .iter()
            .find(|input| {
                input.application_provider
                    && input.language == NativeCompanion::Rust
                    && input.class == action.service
            })
            .ok_or_else(|| Self::failure(&format!(
                "Native menu '{id}' action service '{}' is not a client Rust Service with an application lifetime.",
                action.service
            )))
    }

    fn require_method(id: &str, action: &NativeMenuAction, source: &str) -> Result<(), Failure> {
        let compact = source
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        let signature = format!("fn{}(&self)", action.method);
        compact.contains(&signature).then_some(()).ok_or_else(|| {
            Self::failure(&format!(
                "Native menu '{id}' action method '{}' is not a method of {} that takes only &self.",
                action.method, action.service
            ))
        })
    }

    fn require_model(id: &str, event: &str, root: &Path) -> Result<(), Failure> {
        let declaration = format!("struct {event}");
        let declared = Self::rust_sources(&root.join("client/models"), 0)
            .iter()
            .any(|source| Self::declares(source, &declaration));
        declared.then_some(()).ok_or_else(|| {
            Self::failure(&format!(
                "Native menu '{id}' action event '{event}' is not a Rust model in client/models."
            ))
        })
    }

    fn rust_sources(folder: &Path, depth: usize) -> Vec<String> {
        let entries = std::fs::read_dir(folder)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path());
        entries
            .filter(|_| depth < Self::MAX_DEPTH)
            .flat_map(|path| {
                if path.is_dir() {
                    Self::rust_sources(&path, depth + 1)
                } else {
                    Self::rust_source(&path).into_iter().collect()
                }
            })
            .collect()
    }

    fn rust_source(path: &Path) -> Option<String> {
        (path.extension()? == "rs")
            .then(|| std::fs::read_to_string(path).ok())
            .flatten()
    }

    fn declares(source: &str, declaration: &str) -> bool {
        source.match_indices(declaration).any(|(start, _)| {
            source[start + declaration.len()..]
                .chars()
                .next()
                .is_none_or(|next| !next.is_ascii_alphanumeric() && next != '_')
        })
    }

    fn failure(message: &str) -> Failure {
        let file_name = super::config::WebAppManifest::SOURCE_PATH;
        Failure::one(crate::failure::diagnostic(
            Self::CODE,
            message,
            Some(String::from(Self::HELP)),
            crate::failure::source_span(file_name, 0, file_name.len()),
        ))
    }
}
