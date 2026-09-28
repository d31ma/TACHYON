use super::{Model, ModelField, Record, Shape};

pub(crate) struct WebEventModels;

impl WebEventModels {
    pub(crate) fn models() -> Vec<(String, Model)> {
        vec![
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
