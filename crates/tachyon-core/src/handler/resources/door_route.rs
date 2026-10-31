use super::DoorRoutes;

#[derive(Debug)]
pub(in crate::handler) struct DoorRoute {
    routes: DoorRoutes,
    key: [u8; 32],
}

impl DoorRoute {
    pub(super) const fn new(routes: DoorRoutes, key: [u8; 32]) -> Self {
        Self { routes, key }
    }
}

impl Drop for DoorRoute {
    fn drop(&mut self) {
        if let Ok(mut routes) = self.routes.lock() {
            routes.remove(&self.key);
        }
    }
}
