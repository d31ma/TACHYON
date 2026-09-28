use super::SocketHub;

pub(super) struct SocketSeat {
    pub(super) hub: SocketHub,
    pub(super) id: String,
    pub(super) route: String,
}

impl Drop for SocketSeat {
    fn drop(&mut self) {
        self.hub.leave(&self.id);
    }
}
