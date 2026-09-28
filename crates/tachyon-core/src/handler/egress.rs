mod egress_proxy;
mod egress_target;
mod egress_tunnel;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod egress_tests;

pub(crate) use egress_proxy::EgressProxy;
use egress_target::EgressTarget;
use egress_tunnel::EgressTunnel;
