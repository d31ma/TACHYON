use std::collections::HashMap;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::scroll_page::ScrollPage;
use super::scroll_snapshot::ScrollSnapshot;
use crate::Failure;
use crate::storage::failure;

pub(super) struct ScrollSnapshots {
    entries: HashMap<String, ScrollSnapshot>,
}

impl ScrollSnapshots {
    const MAX_OPEN: usize = 8;
    const LIFETIME: Duration = Duration::from_mins(15);
    const TOKEN_BYTES: usize = 16;

    pub(super) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(super) fn open(&mut self, scope: String, documents: Vec<Value>) -> Result<String, Failure> {
        self.make_room();
        let token = Self::random_token()?;
        let snapshot = ScrollSnapshot {
            scope,
            documents,
            expires_at: Instant::now() + Self::LIFETIME,
        };
        self.entries.insert(token.clone(), snapshot);
        Ok(format!("{token}.0"))
    }

    pub(super) fn page(
        &self,
        cursor: &str,
        scope: &str,
        size: usize,
    ) -> Result<ScrollPage, Failure> {
        let (token, position) = cursor.split_once('.').ok_or_else(Self::expired)?;
        let position = position.parse::<usize>().map_err(|_| Self::expired())?;
        let snapshot = self.live_snapshot(token, scope)?;
        let count = snapshot.documents.len();
        let start = position.min(count);
        let end = start.saturating_add(size).min(count);
        let next_cursor = (end < count).then(|| format!("{token}.{end}"));
        Ok(ScrollPage {
            documents: snapshot.documents[start..end].to_vec(),
            next_cursor,
        })
    }

    fn live_snapshot(&self, token: &str, scope: &str) -> Result<&ScrollSnapshot, Failure> {
        let snapshot = self.entries.get(token).ok_or_else(Self::expired)?;
        let checks = [
            snapshot.expires_at > Instant::now(),
            snapshot.scope == scope,
        ];
        let usable = checks == [true, true];
        usable.then_some(snapshot).ok_or_else(Self::expired)
    }

    fn make_room(&mut self) {
        let now = Instant::now();
        self.entries.retain(|_, snapshot| snapshot.expires_at > now);
        let excess = (self.entries.len() + 1).saturating_sub(Self::MAX_OPEN);
        for token in self.oldest_tokens(excess) {
            self.entries.remove(&token);
        }
    }

    fn oldest_tokens(&self, count: usize) -> Vec<String> {
        let mut ages: Vec<(Instant, String)> = self
            .entries
            .iter()
            .map(|(token, snapshot)| (snapshot.expires_at, token.clone()))
            .collect();
        ages.sort();
        ages.into_iter()
            .take(count)
            .map(|(_, token)| token)
            .collect()
    }

    fn random_token() -> Result<String, Failure> {
        let mut entropy = [0_u8; Self::TOKEN_BYTES];
        getrandom::fill(&mut entropy)
            .map_err(|_| failure("The server could not create a scroll cursor."))?;
        let mut token = String::with_capacity(Self::TOKEN_BYTES * 2);
        for byte in entropy {
            let _ignored = write!(token, "{byte:02x}");
        }
        Ok(token)
    }

    fn expired() -> Failure {
        failure(
            "The scroll cursor expired or belongs to another query; start again from the first page.",
        )
    }
}
