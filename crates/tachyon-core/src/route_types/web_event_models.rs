use super::{Model, ModelField, Record, Shape};

pub(crate) struct WebEventModels;

impl WebEventModels {
    pub(crate) const APPLICATION_STATE: &'static str = "ApplicationStateChanged";

    pub(crate) fn models() -> Vec<(String, Model)> {
        vec![
            Self::record(Self::APPLICATION_STATE, &[("state", Shape::Text)]),
            Self::record("ConnectivityChanged", &[("online", Shape::Boolean)]),
            Self::record("VisibilityChanged", &[("visible", Shape::Boolean)]),
            Self::record(
                "PreferencesChanged",
                &[
                    ("scheme", Shape::Text),
                    ("motion", Shape::Text),
                    ("language", Shape::Text),
                ],
            ),
            Self::record("ClipboardTextRead", &[("text", Shape::Text)]),
            Self::record("ShareFinished", &[]),
            Self::record(
                "NotificationPermissionChanged",
                &[("permission", Shape::Text)],
            ),
            Self::record(
                "PositionFound",
                &[
                    ("latitude", Shape::Number),
                    ("longitude", Shape::Number),
                    ("accuracy", Shape::Number),
                    ("timestamp", Shape::Number),
                ],
            ),
            Self::record("FilePicked", &Self::file_fields(&[])),
            Self::record("FilePickCancelled", &[]),
            Self::record("FileDropped", &Self::file_fields(&[("zone", Shape::Text)])),
            Self::record(
                "FileChunkRead",
                &[
                    ("id", Shape::Text),
                    ("offset", Shape::Number),
                    ("size", Shape::Number),
                    ("data", Shape::Text),
                ],
            ),
            Self::record(
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
            Self::record(
                "PushSubscribed",
                &[
                    ("endpoint", Shape::Text),
                    ("p256dh", Shape::Text),
                    ("auth", Shape::Text),
                    ("expirationTime", Shape::Number),
                ],
            ),
            Self::record("PushUnsubscribed", &[("endpoint", Shape::Text)]),
            Self::record(
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

    fn record(name: &str, fields: &[(&str, Shape)]) -> (String, Model) {
        let mut record = Record::default();
        for (field, shape) in fields {
            record.fields.push(ModelField {
                name: String::from(*field),
                shape: shape.clone(),
                required: true,
            });
        }
        (String::from(name), Model::Record(record))
    }
}
