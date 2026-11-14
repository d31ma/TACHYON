use super::{Model, ModelField, Record, Shape, WebEventSender};

pub(crate) struct WebEventModels;

impl WebEventModels {
    pub(crate) const APPLICATION_STATE: &'static str = "ApplicationStateChanged";
    const MEDIA_GRANTS: [&'static str; 3] = ["camera", "microphone", "screen"];

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
            Self::storage_rows(),
            Self::passkey_rows(),
            Self::media_rows(),
            Self::code_scanner_rows(),
            Self::socket_and_failure_rows(),
            Self::store_rows(),
        ]
        .concat()
    }

    fn store_rows() -> Vec<(WebEventSender, String, Model)> {
        vec![
            Self::record(
                WebEventSender::Page,
                "StorePartitionLoaded",
                &[
                    ("table", Shape::Text),
                    ("partition", Shape::Text),
                    ("rows", Shape::Number),
                ],
            ),
            Self::record(
                WebEventSender::Page,
                "StorePartitionFailed",
                &[
                    ("table", Shape::Text),
                    ("partition", Shape::Text),
                    ("reason", Shape::Text),
                ],
            ),
            Self::record(
                WebEventSender::Page,
                "StoreWriteFailed",
                &[("table", Shape::Text), ("reason", Shape::Text)],
            ),
        ]
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

    fn storage_rows() -> Vec<(WebEventSender, String, Model)> {
        vec![
            Self::record(
                WebEventSender::Page,
                "StorageEstimated",
                &[
                    ("usage", Shape::Number),
                    ("quota", Shape::Number),
                    ("persisted", Shape::Boolean),
                ],
            ),
            Self::record(
                WebEventSender::Grant("persistentStorage"),
                "StoragePersisted",
                &[("persisted", Shape::Boolean)],
            ),
        ]
    }

    fn code_scanner_rows() -> Vec<(WebEventSender, String, Model)> {
        vec![
            Self::record(
                WebEventSender::Grant("camera"),
                "CodeScanned",
                &[("text", Shape::Text)],
            ),
            Self::record(
                WebEventSender::Grant("camera"),
                "ScanFailed",
                &[("reason", Shape::Text)],
            ),
        ]
    }

    fn passkey_rows() -> Vec<(WebEventSender, String, Model)> {
        vec![
            Self::record(
                WebEventSender::Grant("passkeys"),
                "PasskeyFailed",
                &[
                    ("call", Shape::Text),
                    ("reason", Shape::Text),
                    ("message", Shape::Text),
                ],
            ),
            Self::record(
                WebEventSender::Grant("passkeys"),
                "PasskeySupport",
                &[
                    ("platform", Shape::Boolean),
                    ("conditional", Shape::Boolean),
                ],
            ),
            Self::record(
                WebEventSender::Grant("passkeys"),
                "PasskeyCreated",
                &[("credentialJson", Shape::Text)],
            ),
            Self::record(
                WebEventSender::Grant("passkeys"),
                "PasskeyAsserted",
                &[
                    ("credentialJson", Shape::Text),
                    ("conditional", Shape::Boolean),
                ],
            ),
        ]
    }

    fn media_rows() -> Vec<(WebEventSender, String, Model)> {
        vec![
            Self::media(
                "MediaDeviceFound",
                &[
                    ("request", Shape::Text),
                    ("id", Shape::Text),
                    ("kind", Shape::Text),
                    ("label", Shape::Text),
                    ("chosen", Shape::Boolean),
                    ("index", Shape::Number),
                    ("count", Shape::Number),
                ],
            ),
            Self::media("MediaDevicesChanged", &[]),
            Self::media("MediaSpeakerChosen", &[("device", Shape::Text)]),
            Self::media(
                "MediaTrackStarted",
                &[
                    ("track", Shape::Text),
                    ("kind", Shape::Text),
                    ("device", Shape::Text),
                    ("label", Shape::Text),
                    ("width", Shape::Number),
                    ("height", Shape::Number),
                ],
            ),
            Self::media(
                "MediaTrackEnded",
                &[("track", Shape::Text), ("reason", Shape::Text)],
            ),
            Self::media(
                "MediaFailed",
                &[
                    ("track", Shape::Text),
                    ("call", Shape::Text),
                    ("reason", Shape::Text),
                    ("message", Shape::Text),
                ],
            ),
        ]
    }

    fn media(name: &str, fields: &[(&str, Shape)]) -> (WebEventSender, String, Model) {
        Self::record(WebEventSender::AnyGrant(&Self::MEDIA_GRANTS), name, fields)
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
