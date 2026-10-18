use crate::package_architectures::PackageArchitectures;

#[derive(Clone, Copy, Debug)]
pub struct PackagePlan {
    pub package: bool,
    pub release: bool,
    pub architectures: PackageArchitectures,
}
