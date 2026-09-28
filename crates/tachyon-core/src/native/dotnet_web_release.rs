use super::manifest_setting::ManifestSetting;
use std::path::Path;

pub(super) struct DotnetWebRelease {
    ahead_of_time: bool,
}

impl DotnetWebRelease {
    const AHEAD_OF_TIME_SETTING: &'static str = "/tac/dotnet/aot";
    const TRIMMED: &'static str = "    <PublishTrimmed>true</PublishTrimmed>\n";
    const SMALLEST_INTERPRETER: &'static str = "    <TrimMode>full</TrimMode>\n    <EventSourceSupport>false</EventSourceSupport>\n    <DebuggerSupport>false</DebuggerSupport>\n    <MetadataUpdaterSupport>false</MetadataUpdaterSupport>\n    <EmccLinkOptimizationFlag>-Oz</EmccLinkOptimizationFlag>\n";
    const AHEAD_OF_TIME: &'static str = "    <RunAOTCompilation>true</RunAOTCompilation>\n    <WasmStripILAfterAOT>true</WasmStripILAfterAOT>\n";
    const INTERPRETER_NOTE: &'static str = "C#: the trimmed .NET interpreter (full IL trimming, invariant globalization, Emscripten -Oz) for the smallest download; set tac.dotnet.aot to true in client/manifest.json when a service does heavy computing";
    const AHEAD_OF_TIME_NOTE: &'static str = "C#: .NET compiled ahead of time to WebAssembly (tac.dotnet.aot is true): faster for heavy computing, but a larger download";

    pub(super) fn read(snapshot_root: &Path) -> Self {
        let setting = ManifestSetting::read(snapshot_root, Self::AHEAD_OF_TIME_SETTING);
        Self {
            ahead_of_time: setting.and_then(|value| value.as_bool()).unwrap_or(false),
        }
    }

    pub(super) fn project(&self, release_project: &str) -> String {
        let settings = format!(
            "{}{}{}",
            Self::TRIMMED,
            Self::SMALLEST_INTERPRETER,
            self.compiled_settings()
        );
        release_project.replace(Self::TRIMMED, &settings)
    }

    pub(super) fn note(&self) -> String {
        String::from(self.note_text())
    }

    const fn compiled_settings(&self) -> &'static str {
        if self.ahead_of_time {
            Self::AHEAD_OF_TIME
        } else {
            ""
        }
    }

    const fn note_text(&self) -> &'static str {
        if self.ahead_of_time {
            Self::AHEAD_OF_TIME_NOTE
        } else {
            Self::INTERPRETER_NOTE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DotnetWebRelease;

    const PROJECT: &str = "<Project>\n    <PublishTrimmed>true</PublishTrimmed>\n</Project>\n";

    #[test]
    fn a_release_project_trims_fully_and_compiles_ahead_of_time_only_when_asked() {
        let interpreter = DotnetWebRelease {
            ahead_of_time: false,
        };
        let project = interpreter.project(PROJECT);
        assert!(project.contains("<TrimMode>full</TrimMode>"), "{project}");
        assert!(project.contains("<EmccLinkOptimizationFlag>-Oz</EmccLinkOptimizationFlag>"));
        assert!(!project.contains("RunAOTCompilation"), "{project}");
        assert!(interpreter.note().contains("tac.dotnet.aot"));
        let compiled = DotnetWebRelease {
            ahead_of_time: true,
        }
        .project(PROJECT);
        assert!(
            compiled.contains("<RunAOTCompilation>true</RunAOTCompilation>"),
            "{compiled}"
        );
        assert!(compiled.contains("<WasmStripILAfterAOT>true</WasmStripILAfterAOT>"));
    }

    #[test]
    fn the_manifest_setting_turns_ahead_of_time_compiling_on() {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
        let client = root.path().join("client");
        std::fs::create_dir_all(&client).unwrap_or_else(|error| panic!("{error}"));
        std::fs::write(
            client.join("manifest.json"),
            "{\"tac\":{\"dotnet\":{\"aot\":true}}}",
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert!(DotnetWebRelease::read(root.path()).ahead_of_time);
        assert!(!DotnetWebRelease::read(&client).ahead_of_time);
    }
}
