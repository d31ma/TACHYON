pub(super) struct EnvironmentNames;

impl EnvironmentNames {
    pub(super) const ISOLATION: &str = "YON_ISOLATION";
    pub(super) const DRIVER: &str = "YON_FIRECRACKER_DRIVER";
    pub(super) const POOL: &str = "YON_FIRECRACKER_POOL";
    pub(super) const VCPUS: &str = "YON_FIRECRACKER_VCPUS";
    pub(super) const MEMORY: &str = "YON_FIRECRACKER_MEMORY_MIB";
    pub(super) const EGRESS: &str = "YON_FIRECRACKER_EGRESS";
    pub(super) const FIRECRACKER_ONLY: [&str; 5] = [
        Self::DRIVER,
        Self::POOL,
        Self::VCPUS,
        Self::MEMORY,
        Self::EGRESS,
    ];
}
