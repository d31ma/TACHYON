use super::{EgressConfinement, EgressEnforcement, EgressProxy};
use crate::Failure;
use crate::handler::resources::ResourceDoor;
use crate::safety::DeclaredHosts;
use std::sync::Arc;
use tokio::process::Command;

#[derive(Clone, Debug)]
pub(crate) struct EgressSetup {
    proxy: String,
    door: Arc<ResourceDoor>,
    confinement: EgressConfinement,
}

impl EgressSetup {
    pub(crate) async fn start(hosts: DeclaredHosts, required: bool) -> Result<Self, Failure> {
        let enforcement = EgressEnforcement::from_environment()?;
        let guard = EgressProxy::start(hosts).await?;
        let door = ResourceDoor::open().await?;
        let confinement =
            EgressConfinement::prepare(enforcement, required, &[guard.port(), door.port()]).await?;
        Ok(Self {
            proxy: format!("http://{guard}"),
            door,
            confinement,
        })
    }

    pub(crate) fn apply_environment(&self, command: &mut Command) {
        EgressProxy::apply(&self.proxy, command);
    }

    pub(crate) fn confine(&self, command: &mut Command) {
        self.confinement.apply(command);
    }

    pub(in crate::handler) const fn door(&self) -> &Arc<ResourceDoor> {
        &self.door
    }
}
