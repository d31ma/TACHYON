use super::{HandlerCancellation, HandlerEvent};
use crate::handler::HandlerSource;
use tachyon_contracts::HandlerRequest;
use tokio::time::Instant;

pub(super) struct StreamingProcess<'a> {
    pub(super) source: &'a HandlerSource,
    pub(super) request: &'a HandlerRequest,
    pub(super) direct_protocol: bool,
    pub(super) deadline: Instant,
    pub(super) events: tokio::sync::mpsc::Sender<HandlerEvent>,
    pub(super) cancellation: &'a HandlerCancellation,
}
