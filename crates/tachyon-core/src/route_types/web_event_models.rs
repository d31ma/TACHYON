use super::{Model, ModelField, Record, Shape, WebEventSender};

pub(crate) struct WebEventModels;

impl WebEventModels {
    pub(crate) const APPLICATION_STATE: &'static str = "ApplicationStateChanged";

    pub(crate) fn models() -> Vec<(String, Model)> {
        Self::rows().into_iter().map(Self::without_sender).collect()
    }

    pub(crate) fn models_except(own: &[String]) -> Vec<(String, Model)> {
        let mut models = Self::models();
        models.retain(|(name, _)| Self::built_in_only(own, name));
        models
    }

    fn built_in_only(own: &[String], name: &String) -> bool {
        !own.contains(name)
    }

    pub(crate) fn names_except(own: &[String]) -> Vec<String> {
        Self::models_except(own)
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    pub(crate) fn sender(name: &str) -> Option<WebEventSender> {
        Self::rows()
            .into_iter()
            .find(|row| Self::is_row(row, name))
            .map(|(sender, _, _)| sender)
    }

    fn is_row((_, row, _): &(WebEventSender, String, Model), name: &str) -> bool {
        row == name
    }

    fn without_sender((_, name, model): (WebEventSender, String, Model)) -> (String, Model) {
        (name, model)
    }

    fn rows() -> Vec<(WebEventSender, String, Model)> {
        [
            Self::page_rows(),
            Self::prompt_rows(),
            Self::file_rows(),
            Self::push_rows(),
            Self::socket_and_failure_rows(),
        ]
        .concat()
    }

    fn page_rows() -> Vec<(WebEventSender, String, Model)> {
        vec![
            Self::record(
                WebEventSender::Page,
                Self::APPLICATION_STATE,
                &[("state", Shape::Text)],
            ),
            Self::record(
                WebEventSender::Page,
                "ConnectivityChanged",
                &[("online", Shape::Boolean)],
            ),
            Self::record(
                WebEventSender::Page,
                "VisibilityChanged",
                &[("visible", Shape::Boolean)],
            ),
            Self::record(
                WebEventSender::Page,
                "PreferencesChanged",
                &[
                    ("scheme", Shape::Text),
                    ("motion", Shape::Text),
                    ("language", Shape::Text),
                ],
            ),
        ]
    }

    fn prompt_rows() -> Vec<(WebEventSender, String, Model)> {
        vec![
            Self::record(
                WebEventSender::Grant("clipboard"),
                "ClipboardTextRead",
                &[("text", Shape::Text)],
            ),
            Self::record(WebEventSender::Grant("share"), "ShareFinished", &[]),
            Self::record(
                WebEventSender::Grant("notifications"),
                "NotificationPermissionChanged",
                &[("permission", Shape::Text)],
            ),
            Self::record(
                WebEventSender::Grant("geolocation"),
                "PositionFound",
                &[
                    ("latitude", Shape::Number),
                    ("longitude", Shape::Number),
                    ("accuracy", Shape::Number),
                    ("timestamp", Shape::Number),
                ],
            ),
        ]
    }

    fn file_rows() -> Vec<(WebEventSender, String, Model)> {
        vec![
            Self::record(
                WebEventSender::Grant("files"),
                "FilePicked",
                &Self::file_fields(&[]),
            ),
            Self::record(WebEventSender::Grant("files"), "FilePickCancelled", &[]),
            Self::record(
                WebEventSender::Grant("files"),
                "FileDropped",
                &Self::file_fields(&[("zone", Shape::Text)]),
            ),
            Self::record(
                WebEventSender::Grant("files"),
                "FileChunkRead",
                &[
                    ("id", Shape::Text),
                    ("offset", Shape::Number),
                    ("size", Shape::Number),
                    ("data", Shape::Text),
                ],
            ),
            Self::record(
                WebEventSender::Grant("files"),
                "UploadFinished",
                &[
                    ("upload", Shape::Text),
                    ("id", Shape::Text),
                    ("status", Shape::Number),
                    ("body", Shape::Text),
                    ("truncated", Shape::Boolean),
                    ("error", Shape::Text),
                ],
            ),
        ]
    }

    fn push_rows() -> Vec<(WebEventSender, String, Model)> {
        vec![
            Self::record(
                WebEventSender::Grant("push"),
                "PushSubscribed",
                &[
                    ("endpoint", Shape::Text),
                    ("p256dh", Shape::Text),
                    ("auth", Shape::Text),
                    ("expirationTime", Shape::Number),
                ],
            ),
            Self::record(
                WebEventSender::Grant("push"),
                "PushUnsubscribed",
                &[("endpoint", Shape::Text)],
            ),
            Self::record(
                WebEventSender::Grant("push"),
                "PushReceived",
                &[
                    ("tag", Shape::Text),
                    ("path", Shape::Text),
                    ("data", Shape::Text),
                ],
            ),
        ]
    }

    fn socket_and_failure_rows() -> Vec<(WebEventSender, String, Model)> {
        vec![
            Self::record(
                WebEventSender::Socket,
                "SocketOpened",
                &[("path", Shape::Text), ("reconnected", Shape::Boolean)],
            ),
            Self::record(
                WebEventSender::Socket,
                "SocketClosed",
                &[
                    ("path", Shape::Text),
                    ("code", Shape::Number),
                    ("reconnecting", Shape::Boolean),
                ],
            ),
            Self::record(
                WebEventSender::Page,
                "BrowserCallFailed",
                &[
                    ("api", Shape::Text),
                    ("call", Shape::Text),
                    ("message", Shape::Text),
                ],
            ),
        ]
    }

    pub(crate) fn names() -> Vec<String> {
        Self::models().into_iter().map(|(name, _)| name).collect()
    }

    pub(crate) fn page_names() -> Vec<String> {
        Self::names()
            .into_iter()
            .filter(|name| name != Self::APPLICATION_STATE)
            .collect()
    }

    fn file_fields(extra: &[(&'static str, Shape)]) -> Vec<(&'static str, Shape)> {
        let mut fields = extra.to_vec();
        fields.extend([
            ("id", Shape::Text),
            ("name", Shape::Text),
            ("size", Shape::Number),
            ("media", Shape::Text),
            ("sha256", Shape::Text),
            ("index", Shape::Number),
            ("count", Shape::Number),
        ]);
        fields
    }

    fn record(
        sender: WebEventSender,
        name: &str,
        fields: &[(&str, Shape)],
    ) -> (WebEventSender, String, Model) {
        let mut record = Record::default();
        for (field, shape) in fields {
            record.fields.push(ModelField {
                name: String::from(*field),
                shape: shape.clone(),
                required: true,
            });
        }
        (sender, String::from(name), Model::Record(record))
    }
}
