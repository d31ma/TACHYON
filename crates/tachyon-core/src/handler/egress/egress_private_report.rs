use crate::sorted_json::SortedJson;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub(super) struct EgressPrivateReport {
    hosts: Mutex<HashSet<String>>,
}

impl EgressPrivateReport {
    const EVENT: &'static str = "handler.egress_private";

    pub(super) fn note(&self, host: &str, address: SocketAddr) {
        let _reported = self.first_time(host).then(|| Self::report(host, address));
    }

    fn first_time(&self, host: &str) -> bool {
        self.hosts
            .lock()
            .is_ok_and(|mut hosts| hosts.insert(host.to_owned()))
    }

    fn report(host: &str, address: SocketAddr) {
        eprintln!(
            "{}",
            SortedJson::text(&serde_json::json!({
                "event": Self::EVENT,
                "host": host,
                "address": address.ip().to_string(),
            }))
        );
    }
}
