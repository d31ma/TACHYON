use super::{HandlerCancellation, HandlerEvent};
use crate::handler::HandlerSource;
use crate::handler::resources::ResourceAccess;
use tachyon_contracts::HandlerRequest;
use tokio::sync::mpsc;
use tokio::time::Instant;

pub(super) struct WarmStreamRequest<'a> {
    pub(super) source: &'a HandlerSource,
    pub(super) request: &'a HandlerRequest,
    pub(super) resource_access: Option<&'a ResourceAccess>,
    pub(super) deadline: Instant,
    pub(super) events: mpsc::Sender<HandlerEvent>,
    pub(super) cancellation: &'a HandlerCancellation,
}
