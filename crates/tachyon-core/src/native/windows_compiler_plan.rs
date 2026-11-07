use super::windows_architecture::WindowsArchitecture;
use std::path::Path;

pub(super) struct WindowsCompilerPlan<'plan> {
    pub(super) sdk: &'plan Path,
    pub(super) source: &'plan str,
    pub(super) executable: &'plan str,
    pub(super) resource: Option<&'plan str>,
    pub(super) architecture: WindowsArchitecture,
}

impl WindowsCompilerPlan<'_> {
    const COMPILER_OPTIONS: [&'static str; 6] =
        ["/nologo", "/std:c17", "/W4", "/WX", "/O2", "/Brepro"];
    const REPRODUCIBLE_LINK: &'static str = "/Brepro";
    const SDK_HEADERS: &'static str = "build/native/include";
    const NO_RESOURCE: &'static str = "/nologo";

    pub(super) fn arguments(&self, linker_arguments: &[&str]) -> Vec<String> {
        let mut arguments = Self::COMPILER_OPTIONS.map(String::from).to_vec();
        arguments.extend([
            format!("/I{}", self.sdk.join(Self::SDK_HEADERS).display()),
            format!("/Fo{}\\", super::windows::object_directory(self.executable)),
            self.source.to_owned(),
            self.resource.unwrap_or(Self::NO_RESOURCE).to_owned(),
            format!("/Fe:{}", self.executable),
            String::from("/link"),
            format!(
                "/LIBPATH:{}",
                self.sdk
                    .join(self.architecture.webview2_library_folder())
                    .display()
            ),
            String::from(self.architecture.linker_machine()),
            String::from(Self::REPRODUCIBLE_LINK),
        ]);
        arguments.extend(linker_arguments.iter().copied().map(String::from));
        arguments
    }
}
