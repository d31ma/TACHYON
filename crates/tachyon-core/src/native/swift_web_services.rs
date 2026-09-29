use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};
use swift_wasm_program::SwiftWasmProgram;

mod swift_wasm_program;

pub(crate) struct SwiftWebServices;

impl SwiftWebServices {
    const PRELUDE: &'static str = concat!(
        include_str!("prelude.swift"),
        include_str!("prelude_web.swift"),
        include_str!("swift_web_services/TacJSONReader.swift"),
        include_str!("swift_web_services/TacValueDecoder.swift"),
        include_str!("swift_web_services/TacSingleValueDecoder.swift"),
        include_str!("swift_web_services/TacKeyedValueDecoder.swift"),
        include_str!("swift_web_services/TacListValueDecoder.swift"),
        include_str!("swift_web_services/TacEncodedNode.swift"),
        include_str!("swift_web_services/TacValueEncoder.swift"),
        include_str!("swift_web_services/TacSingleValueEncoder.swift"),
        include_str!("swift_web_services/TacKeyedValueEncoder.swift"),
        include_str!("swift_web_services/TacListValueEncoder.swift"),
        include_str!("swift_web_services/TacWebStorage.swift"),
        include_str!("swift_web_services/TacWebHost.swift"),
        include_str!("swift_web_services/TacAwaited.swift")
    );
    const BEHAVIOR_PLATFORM: &'static str = concat!(
        include_str!("swift_web_services/BehaviorPlatform.swift"),
        include_str!("swift_web_services/BehaviorLock.swift"),
        include_str!("swift_web_services/BehaviorBrowserStorage.swift")
    );

    pub(crate) fn sdk_installed() -> bool {
        SwiftWasmProgram::sdk().is_ok()
    }

    pub(crate) fn compiler() -> std::ffi::OsString {
        SwiftWasmProgram::compiler()
    }

    pub(crate) async fn build(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
        profile: crate::BuildProfile,
    ) -> Result<(Vec<(String, Vec<u8>)>, Option<String>), Failure> {
        let companions =
            NativeCompanionInput::web_services(snapshot_root, NativeCompanion::Swift, services);
        let authored = super::registry::source(&companions, NativeCompanion::Swift)?;
        Self::compiled(authored, profile).await
    }

    async fn compiled(
        authored: Option<String>,
        profile: crate::BuildProfile,
    ) -> Result<(Vec<(String, Vec<u8>)>, Option<String>), Failure> {
        let Some(authored) = authored else {
            return Ok((Vec::new(), None));
        };
        let authored =
            crate::behavior::SwiftPlatform::in_browser(&authored, Self::BEHAVIOR_PLATFORM);
        let program = format!("{authored}\n{}", Self::PRELUDE);
        let built = SwiftWasmProgram::compile(&program, profile).await?;
        let note = built.note(profile).map(String::from);
        Ok((built.published(), note))
    }
}
