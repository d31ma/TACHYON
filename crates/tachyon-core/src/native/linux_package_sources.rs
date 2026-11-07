use super::config::NativeApplication;
use super::linux_package_names::LinuxPackageNames;
use std::path::Path;

pub(super) struct LinuxPackageSources<'build> {
    pub(super) application: &'build NativeApplication,
    pub(super) names: &'build LinuxPackageNames,
    pub(super) bundle: &'build Path,
    pub(super) web_bundle: &'build Path,
}
