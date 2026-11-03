use super::TopicDelivery;
use crate::handler::source::HandlerSource;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct TopicListeners {
    by_topic: BTreeMap<String, Vec<(HandlerSource, Vec<String>)>>,
    by_source: BTreeMap<String, Vec<(String, String)>>,
}

impl TopicListeners {
    pub(crate) fn from_sources<'a>(sources: impl Iterator<Item = &'a HandlerSource>) -> Self {
        let mut listeners = Self::default();
        listeners.add_sources(Self::listening_sources(sources));
        listeners
    }

    fn add_sources(&mut self, sources: Vec<&HandlerSource>) {
        let mut taken: BTreeSet<(String, String, String)> = BTreeSet::new();
        for source in sources {
            self.add_source(source, &mut taken);
        }
    }

    fn listening_sources<'a>(
        sources: impl Iterator<Item = &'a HandlerSource>,
    ) -> Vec<&'a HandlerSource> {
        let mut sources: Vec<&HandlerSource> = sources
            .filter(|source| !source.topic_listeners().is_empty())
            .collect();
        sources.sort_by(|left, right| left.relative_path().cmp(right.relative_path()));
        sources.dedup_by(|left, right| left.relative_path() == right.relative_path());
        sources
    }

    fn add_source(
        &mut self,
        source: &HandlerSource,
        taken: &mut BTreeSet<(String, String, String)>,
    ) {
        let owned = Self::claimed_keys(source, taken);
        self.by_source.insert(
            source.relative_path().to_owned(),
            source.topic_listeners().to_vec(),
        );
        for (topic, keys) in owned {
            self.by_topic
                .entry(topic)
                .or_default()
                .push((source.clone(), keys));
        }
    }

    fn claimed_keys(
        source: &HandlerSource,
        taken: &mut BTreeSet<(String, String, String)>,
    ) -> BTreeMap<String, Vec<String>> {
        let language = Self::language_of(source);
        let mut owned: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (topic, key) in source.topic_listeners() {
            Self::claim_key(&language, topic, key, taken, &mut owned);
        }
        owned
    }

    fn claim_key(
        language: &str,
        topic: &str,
        key: &str,
        taken: &mut BTreeSet<(String, String, String)>,
        owned: &mut BTreeMap<String, Vec<String>>,
    ) {
        let identity = (language.to_owned(), topic.to_owned(), key.to_owned());
        if taken.insert(identity) {
            owned
                .entry(topic.to_owned())
                .or_default()
                .push(key.to_owned());
        }
    }

    pub(super) fn targets(&self, delivery: &TopicDelivery) -> Vec<(HandlerSource, Vec<String>)> {
        let origin_language = Self::extension_of(&delivery.origin);
        let ran_locally: Vec<&String> = self
            .by_source
            .get(&delivery.origin)
            .into_iter()
            .flatten()
            .filter(|(topic, _)| *topic == delivery.topic)
            .map(|(_, key)| key)
            .collect();
        self.by_topic
            .get(&delivery.topic)
            .into_iter()
            .flatten()
            .filter_map(|(source, keys)| Self::target(source, keys, &origin_language, &ran_locally))
            .collect()
    }

    fn target(
        source: &HandlerSource,
        keys: &[String],
        origin_language: &str,
        ran_locally: &[&String],
    ) -> Option<(HandlerSource, Vec<String>)> {
        let same_language = Self::language_of(source) == origin_language;
        let keys: Vec<String> = keys
            .iter()
            .filter(|key| Self::still_needs_delivery(same_language, ran_locally, key))
            .cloned()
            .collect();
        let has_keys = !keys.is_empty();
        has_keys.then(|| (source.clone(), keys))
    }

    fn still_needs_delivery(same_language: bool, ran_locally: &[&String], key: &String) -> bool {
        !(same_language && ran_locally.contains(&key))
    }

    fn language_of(source: &HandlerSource) -> String {
        Self::extension_of(source.relative_path())
    }

    fn extension_of(path: &str) -> String {
        path.rsplit_once('.')
            .map_or("", |(_, extension)| extension)
            .to_owned()
    }
}
