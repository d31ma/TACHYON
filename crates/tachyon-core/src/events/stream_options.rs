use crate::behavior::{ReconnectPolicy, StreamEventName, StreamQuery};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct StreamOptions {
    pub(crate) path: String,
    pub(crate) reconnect: Option<ReconnectPolicy>,
    pub(crate) event: Option<StreamEventName>,
    pub(crate) query: Option<StreamQuery>,
}

impl StreamOptions {
    pub(crate) fn addressed(&self, full: &str) -> String {
        let queried = self.queried(full);
        let named = self.named(&queried);
        self.reconnected(&named)
    }

    fn queried(&self, full: &str) -> String {
        self.query
            .as_ref()
            .map_or_else(|| full.to_owned(), |query| query.addressed(full))
    }

    fn named(&self, path: &str) -> String {
        self.event
            .as_ref()
            .map_or_else(|| path.to_owned(), |event| event.addressed(path))
    }

    fn reconnected(&self, path: &str) -> String {
        self.reconnect
            .map_or_else(|| path.to_owned(), |policy| policy.addressed(path))
    }
}
