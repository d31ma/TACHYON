use std::fs;
use std::path::Path;

pub(super) struct VisibilityOff;

impl VisibilityOff {
    const CLIENT: (&'static str, &'static str) = (
        "client/manifest.json",
        r#"{"tac":{"visibility":{"check":false}}}"#,
    );
    const SERVER: (&'static str, &'static str) = (
        "server/application.json",
        r#"{"visibility":{"check":false}}"#,
    );

    pub(super) fn write(root: &Path) {
        Self::write_settings(root, Self::CLIENT);
        Self::write_settings(root, Self::SERVER);
    }

    fn write_settings(root: &Path, (relative, settings): (&str, &str)) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        fs::write(path, settings).expect("settings");
    }
}
