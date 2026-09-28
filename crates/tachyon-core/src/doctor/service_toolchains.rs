use super::Toolchain;
use super::known_toolchains::KnownToolchains;
use crate::project::NativeCompanion;
use crate::tac_web::WebLanguage;
use crate::template::{ServiceDefinition, ServiceRegistry};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct ServiceToolchains;

impl ServiceToolchains {
    const NATIVE_LANGUAGES: [NativeCompanion; 4] = [
        NativeCompanion::CSharp,
        NativeCompanion::Kotlin,
        NativeCompanion::Rust,
        NativeCompanion::Swift,
    ];

    const OPTIMIZED_WEB_LANGUAGES: [WebLanguage; 3] =
        [WebLanguage::Rust, WebLanguage::Go, WebLanguage::Swift];

    pub(super) fn list(root: &Path, services: &ServiceRegistry) -> Vec<Toolchain> {
        let scripts = Self::script_extensions(services);
        let mut toolchains = Vec::new();
        toolchains.extend(
            scripts
                .contains("ts")
                .then(|| KnownToolchains::typescript(root)),
        );
        toolchains.extend(
            Self::NATIVE_LANGUAGES
                .into_iter()
                .filter(|language| Self::uses_native_service(services, *language))
                .map(KnownToolchains::native_service),
        );
        toolchains.extend(
            Self::uses_web_service(services, WebLanguage::Rust).then(KnownToolchains::web_assembly),
        );
        toolchains.extend(
            Self::uses_web_service(services, WebLanguage::CSharp).then(KnownToolchains::dotnet_web),
        );
        toolchains.extend(
            Self::uses_web_service(services, WebLanguage::Go).then(KnownToolchains::go_web),
        );
        toolchains.extend(
            Self::uses_web_service(services, WebLanguage::Kotlin).then(KnownToolchains::kotlin_web),
        );
        toolchains.extend(
            Self::uses_web_service(services, WebLanguage::Go).then(KnownToolchains::tinygo),
        );
        toolchains.extend(Self::uses_optimized_web_service(services).then(KnownToolchains::binaryen));
        toolchains.extend(
            Self::uses_web_service(services, WebLanguage::Swift).then(KnownToolchains::swift_web),
        );
        toolchains.extend(
            Self::uses_web_service(services, WebLanguage::Ruby)
                .then(crate::tac_ruby::RubyRuntimeBuild::toolchains)
                .unwrap_or_default(),
        );
        toolchains.extend(KnownToolchains::fylo(root));
        toolchains.extend(scripts.contains("dart").then(KnownToolchains::dart));
        toolchains
    }

    fn script_extensions(services: &ServiceRegistry) -> BTreeSet<&str> {
        services
            .iter()
            .filter_map(|(_, service)| Self::script_extension(service))
            .collect()
    }

    fn script_extension(service: &ServiceDefinition) -> Option<&str> {
        service.script_path()?.extension()?.to_str()
    }

    fn uses_native_service(services: &ServiceRegistry, language: NativeCompanion) -> bool {
        services.iter().any(|(_, service)| {
            service
                .native_companions()
                .iter()
                .any(|(used, _)| *used == language)
        })
    }

    fn uses_optimized_web_service(services: &ServiceRegistry) -> bool {
        Self::OPTIMIZED_WEB_LANGUAGES
            .into_iter()
            .any(|language| Self::uses_web_service(services, language))
    }

    fn uses_web_service(services: &ServiceRegistry, language: WebLanguage) -> bool {
        services.iter().any(|(_, service)| {
            service
                .web_assembly()
                .is_some_and(|(used, _)| used == language)
        })
    }
}
