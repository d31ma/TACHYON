use crate::Failure;
use std::path::Path;
use tachyon_contracts::NativeTarget;

pub(crate) struct NativeApplicationLifetime;

impl NativeApplicationLifetime {
    pub(crate) fn supported(path: &Path, value: &str) -> bool {
        Self::value(path, value)
    }

    pub(crate) fn advice(value: &str) -> &'static str {
        Self::message(value)
    }

    pub(crate) fn target(
        path: &Path,
        application: Option<usize>,
        target: Option<NativeTarget>,
    ) -> Result<(), Failure> {
        application
            .map(|at| Self::target_at(path, at, target))
            .transpose()
            .map(|_| ())
    }

    fn value(path: &Path, value: &str) -> bool {
        [
            ["singleton", "prototype"].contains(&value),
            (value, Self::rust_service(path)) == ("application", true),
        ]
        .contains(&true)
    }

    fn rust_service(path: &Path) -> bool {
        let extension = path.extension().and_then(|value| value.to_str());
        let realm = crate::annotation_rules::Realm::of(path);
        let folder = Self::client_folder(path);
        (extension, realm, folder)
            == (
                Some("rs"),
                Some(crate::annotation_rules::Realm::Tac),
                Some(std::ffi::OsStr::new("services")),
            )
    }

    fn client_folder(path: &Path) -> Option<&std::ffi::OsStr> {
        path.components()
            .skip_while(|part| part.as_os_str() != "client")
            .nth(1)
            .map(std::path::Component::as_os_str)
    }

    fn message(value: &str) -> &'static str {
        match value {
            "application" => {
                "Application lifetime requires a client Rust Service and a native macOS build; other languages, roles and realms are unsupported."
            }
            _ => "Lifetime.value must be singleton or prototype",
        }
    }

    fn target_at(path: &Path, at: usize, target: Option<NativeTarget>) -> Result<(), Failure> {
        (target == Some(NativeTarget::Macos))
            .then_some(())
            .ok_or_else(|| {
                Failure::one(crate::failure::diagnostic(
                    1406,
                    "Application lifetime is supported only by native macOS/Rust; browser and other native targets cannot own this provider.",
                    None,
                    crate::failure::source_span(&path.to_string_lossy(), at, at + "Lifetime".len()),
                ))
            })
    }
}
