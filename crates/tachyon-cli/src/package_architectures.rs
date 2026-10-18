use tachyon_core::{MacOsArchitecture, WindowsArchitecture};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PackageArchitectures {
    pub windows: WindowsArchitecture,
    pub macos: MacOsArchitecture,
}
