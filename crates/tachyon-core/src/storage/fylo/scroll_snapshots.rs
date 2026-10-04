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
        let now = Instant::now();
        self.entries.retain(|_, snapshot| snapshot.expires_at > now);
        if self.entries.len() >= Self::MAX_OPEN {
            self.remove_oldest();
        }
        let token = Self::random_token()?;
        let snapshot = ScrollSnapshot {
            scope,
            documents,
            expires_at: now + Self::LIFETIME,
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
        let (token, position) = Self::parse_cursor(cursor)?;
        let snapshot = self
            .entries
            .get(token)
            .filter(|snapshot| snapshot.expires_at > Instant::now() && snapshot.scope == scope)
            .ok_or_else(Self::expired)?;
        let start = position.min(snapshot.documents.len());
        let end = start.saturating_add(size).min(snapshot.documents.len());
        let next_cursor = if end < snapshot.documents.len() {
            Some(format!("{token}.{end}"))
        } else {
            None
        };
        Ok(ScrollPage {
            documents: snapshot.documents[start..end].to_vec(),
            next_cursor,
        })
    }

    fn parse_cursor(cursor: &str) -> Result<(&str, usize), Failure> {
        let (token, position) = cursor.split_once('.').ok_or_else(Self::expired)?;
        let position = position.parse::<usize>().map_err(|_| Self::expired())?;
        Ok((token, position))
    }

    fn remove_oldest(&mut self) {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, snapshot)| snapshot.expires_at)
            .map(|(token, _)| token.clone());
        if let Some(token) = oldest {
            self.entries.remove(&token);
        }
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
