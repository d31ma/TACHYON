use super::{SocketInput, SocketOutput};
use tokio::sync::mpsc;

pub(crate) struct SocketChannels {
    pub(crate) input: mpsc::Receiver<SocketInput>,
    pub(crate) output: mpsc::Sender<SocketOutput>,
}
