use crate::Failure;
use crate::handler::readiness::RuntimeRequirement;
use crate::handler::{EnvironmentPolicy, HandlerRuntimePrograms, RuntimeProbe, RuntimeProbeResult};
use std::sync::Arc;
use tokio::sync::OnceCell;

#[derive(Clone, Debug, Default)]
pub(crate) struct NodeProxyGate {
    verdict: Arc<OnceCell<Result<(), Failure>>>,
}

impl NodeProxyGate {
    pub(crate) async fn check(
        &self,
        runtimes: &HandlerRuntimePrograms,
        environment: &EnvironmentPolicy,
    ) -> Result<(), Failure> {
        let verdict = self
            .verdict
            .get_or_init(|| Self::probe(runtimes, environment))
            .await;
        verdict.clone()
    }

    async fn probe(
        runtimes: &HandlerRuntimePrograms,
        environment: &EnvironmentPolicy,
    ) -> Result<(), Failure> {
        let requirement = RuntimeRequirement::node_proxy_switch(runtimes);
        let results = RuntimeProbe::all(vec![requirement], environment).await?;
        RuntimeProbeResult::failure(&results).map_or(Ok(()), Err)
    }
}
