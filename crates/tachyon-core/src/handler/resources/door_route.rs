use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(in crate::handler) struct DoorRoute {
    routes: Arc<Mutex<HashMap<[u8; 32], SocketAddr>>>,
    key: [u8; 32],
}

impl DoorRoute {
    pub(super) const fn new(
        routes: Arc<Mutex<HashMap<[u8; 32], SocketAddr>>>,
        key: [u8; 32],
    ) -> Self {
        Self { routes, key }
    }
}

impl Drop for DoorRoute {
    fn drop(&mut self) {
        let _removed = self
            .routes
            .lock()
            .map(|mut routes| routes.remove(&self.key));
    }
}
