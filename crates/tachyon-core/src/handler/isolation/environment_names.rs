pub(super) struct EnvironmentNames;

impl EnvironmentNames {
    pub(super) const ISOLATION: &str = "YON_ISOLATION";
    pub(super) const DRIVER: &str = "YON_FIRECRACKER_DRIVER";
    pub(super) const POOL: &str = "YON_FIRECRACKER_POOL";
    pub(super) const VCPUS: &str = "YON_FIRECRACKER_VCPUS";
    pub(super) const MEMORY: &str = "YON_FIRECRACKER_MEMORY_MIB";
    pub(super) const EGRESS: &str = "YON_FIRECRACKER_EGRESS";
    pub(super) const WARM_MAX_REQUESTS: &str = "YON_WARM_MAX_REQUESTS";
    pub(super) const WARM_IDLE_SECONDS: &str = "YON_WARM_IDLE_SECONDS";
    const FIRECRACKER_ONLY: [&str; 5] = [
        Self::DRIVER,
        Self::POOL,
        Self::VCPUS,
        Self::MEMORY,
        Self::EGRESS,
    ];
    const WARM_ONLY: [&str; 2] = [Self::WARM_MAX_REQUESTS, Self::WARM_IDLE_SECONDS];

    pub(super) fn foreign_to(mode: &str) -> Vec<&'static str> {
        let firecracker = Self::FIRECRACKER_ONLY.as_slice();
        let warm = Self::WARM_ONLY.as_slice();
        let owned = [("firecracker", firecracker), ("warm", warm)];
        owned
            .into_iter()
            .filter(|(owner, _)| *owner != mode)
            .flat_map(|(_, names)| names.iter().copied())
            .collect()
    }
}
