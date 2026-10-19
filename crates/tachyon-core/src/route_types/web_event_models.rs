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
