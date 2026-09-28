use std::fs;
use std::path::Path;

pub(super) struct VisibilityOff;

impl VisibilityOff {
    const SETTINGS: [(&'static str, &'static str); 2] = [
        (
            "client/manifest.json",
            r#"{"tac":{"visibility":{"check":false}}}"#,
        ),
        (
            "server/application.json",
            r#"{"visibility":{"check":false}}"#,
        ),
    ];

    pub(super) fn write(root: &Path) {
        for (relative, settings) in Self::SETTINGS {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("folders");
            fs::write(path, settings).expect("settings");
        }
    }
}
