use tokio::sync::mpsc;

#[derive(Debug)]
pub(super) struct SocketPeer {
    pub(super) route: String,
    pub(super) sender: mpsc::Sender<String>,
}
