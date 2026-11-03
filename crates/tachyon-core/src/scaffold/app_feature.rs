use super::{Choice, TemplateFiles};

type FeatureRow = (&'static str, &'static str, bool, &'static str);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppFeature {
    Storage,
    Sockets,
    Topics,
    Streams,
    HttpClient,
    ErrorHandlers,
}

impl AppFeature {
    const ROWS: [FeatureRow; 6] = [
        (
            "storage",
            "Storage: a Store that reads the database",
            true,
            "",
        ),
        (
            "sockets",
            "Sockets: a chat room over a WebSocket",
            true,
            "chat-room",
        ),
        (
            "topics",
            "Topics: news that reaches every open page",
            true,
            "news-feed",
        ),
        (
            "streams",
            "Streams: one answer that sends many messages",
            true,
            "ticks-stream",
        ),
        (
            "http-client",
            "HTTP client: a page that calls the server",
            false,
            "hello-card",
        ),
        (
            "error-handlers",
            "Error handlers: turn an error into a clear answer",
            true,
            "",
        ),
    ];

    #[must_use]
    pub fn needs_server(self) -> bool {
        self.row().2
    }

    pub(crate) fn component(self) -> &'static str {
        self.row().3
    }

    pub(crate) fn files(self, typescript: bool) -> Vec<(String, String)> {
        let folder = format!(
            "features/{}/{}",
            self.flag_name(),
            Self::language_folder(typescript)
        );
        TemplateFiles::under(&folder, "")
    }

    fn language_folder(typescript: bool) -> &'static str {
        if typescript {
            return "ts";
        }
        "js"
    }

    fn row(self) -> FeatureRow {
        Self::ROWS[self as usize]
    }
}

impl Choice for AppFeature {
    const ALL: &'static [Self] = &[
        Self::Storage,
        Self::Sockets,
        Self::Topics,
        Self::Streams,
        Self::HttpClient,
        Self::ErrorHandlers,
    ];
    const QUESTION: &'static str =
        "Which examples do you want? You can choose more than one, or none.";

    fn names(self) -> &'static str {
        self.row().0
    }

    fn label(self) -> &'static str {
        self.row().1
    }
}
