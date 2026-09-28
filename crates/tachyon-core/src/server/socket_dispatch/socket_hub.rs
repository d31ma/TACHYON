use super::{SocketPeer, SocketSeat};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::sync::mpsc;

#[derive(Clone, Debug, Default)]
pub(in crate::server) struct SocketHub {
    sessions: Arc<Mutex<BTreeMap<String, SocketPeer>>>,
}

impl SocketHub {
    const MAX_SESSIONS: usize = 32;
    const PEER_QUEUE: usize = 16;

    pub(super) fn join(&self, route: &str) -> Option<(SocketSeat, mpsc::Receiver<String>)> {
        let mut sessions = self.lock();
        let has_room = sessions.len() < Self::MAX_SESSIONS;
        has_room.then(|| self.seat(&mut sessions, route))
    }

    pub(super) fn leave(&self, id: &str) {
        self.lock().remove(id);
    }

    pub(super) fn ids(&self, route: &str) -> Vec<String> {
        self.lock()
            .iter()
            .filter(|(_, peer)| peer.route == route)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub(super) fn deliver(&self, route: &str, id: &str, data: String) {
        let mut sessions = self.lock();
        let accepted = sessions
            .get(id)
            .filter(|peer| peer.route == route)
            .map(|peer| peer.sender.try_send(data).is_ok());
        Self::drop_when_full(&mut sessions, id, accepted);
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, SocketPeer>> {
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn seat(
        &self,
        sessions: &mut BTreeMap<String, SocketPeer>,
        route: &str,
    ) -> (SocketSeat, mpsc::Receiver<String>) {
        let id = crate::ttid::generate();
        let (sender, receiver) = mpsc::channel(Self::PEER_QUEUE);
        let peer = SocketPeer {
            route: route.to_owned(),
            sender,
        };
        sessions.insert(id.clone(), peer);
        let seat = SocketSeat {
            hub: self.clone(),
            id,
            route: route.to_owned(),
        };
        (seat, receiver)
    }

    fn drop_when_full(
        sessions: &mut BTreeMap<String, SocketPeer>,
        id: &str,
        accepted: Option<bool>,
    ) {
        if accepted == Some(false) {
            sessions.remove(id);
        }
    }
}
