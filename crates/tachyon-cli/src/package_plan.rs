use tachyon_core::WindowsArchitecture;

#[derive(Clone, Copy, Debug)]
pub struct PackagePlan {
    pub package: bool,
    pub windows_architecture: WindowsArchitecture,
}
