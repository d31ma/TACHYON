use super::config::{WebAppManifest, config_failure};
use crate::Failure;
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevelopmentOrigin {
    target: NativeTarget,
    port: u16,
    origin: String,
}

impl DevelopmentOrigin {
    const LOOPBACK: &'static str = "127.0.0.1";
    const EMULATOR_LOOPBACK: &'static str = "localhost";
    const TAC_KEY: &'static str = "tac";
    const YON_KEY: &'static str = "yon";
    const ORIGIN_KEY: &'static str = "origin";
    const STAGED_NAME: &'static str = "manifest.json.development";

    #[must_use]
    pub fn new(target: NativeTarget, port: u16) -> Self {
        let host = match target {
            NativeTarget::Android => Self::EMULATOR_LOOPBACK,
            NativeTarget::Macos
            | NativeTarget::Ios
            | NativeTarget::Linux
            | NativeTarget::Windows => Self::LOOPBACK,
        };
        Self {
            target,
            port,
            origin: format!("http://{host}:{port}"),
        }
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub const fn target(&self) -> NativeTarget {
        self.target
    }

    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin
    }

    #[must_use]
    pub const fn app_origin(&self) -> &'static str {
        match self.target {
            NativeTarget::Macos | NativeTarget::Ios => "tachyon-app://bundle",
            NativeTarget::Android => "https://appassets.tachyon.local",
            NativeTarget::Windows => "https://tachyon.local",
            NativeTarget::Linux => "tachyon://app",
        }
    }

    pub fn declare_in(&self, snapshot_root: &Path) -> Result<Option<String>, Failure> {
        let path = snapshot_root.join(WebAppManifest::SOURCE_PATH);
        let document = Self::read(&path)?;
        let replaced = Self::declared_origin(&document).filter(|declared| *declared != self.origin);
        Self::write(&path, &self.with_origin(document))?;
        Ok(replaced.map(|declared| self.replaced_note(&declared)))
    }

    fn replaced_note(&self, declared: &str) -> String {
        format!(
            "Note: the preview app calls Yon on the development server {} instead of tac.yon.origin {declared}. Release builds keep {declared}.",
            self.origin
        )
    }

    fn read(path: &Path) -> Result<Value, Failure> {
        match fs::read_to_string(path) {
            Ok(source) => serde_json::from_str(crate::without_bom(&source)).map_err(|error| {
                config_failure(&format!(
                    "{} is invalid: {error}",
                    WebAppManifest::SOURCE_PATH
                ))
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(Value::Object(Map::new()))
            }
            Err(error) => Err(Self::io_failure(path, &error)),
        }
    }

    fn declared_origin(document: &Value) -> Option<String> {
        document
            .pointer("/tac/yon/origin")
            .and_then(Value::as_str)
            .map(String::from)
    }

    fn with_origin(&self, document: Value) -> Value {
        let mut root = Self::into_object(document);
        let mut tac = Self::into_object(root.remove(Self::TAC_KEY).unwrap_or_default());
        let mut yon = Self::into_object(tac.remove(Self::YON_KEY).unwrap_or_default());
        yon.insert(
            String::from(Self::ORIGIN_KEY),
            Value::from(self.origin.clone()),
        );
        tac.insert(String::from(Self::YON_KEY), Value::Object(yon));
        root.insert(String::from(Self::TAC_KEY), Value::Object(tac));
        Value::Object(root)
    }

    fn into_object(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(map) => map,
            _ => Map::new(),
        }
    }

    fn write(path: &Path, document: &Value) -> Result<(), Failure> {
        let folder = path.parent().unwrap_or(path);
        fs::create_dir_all(folder).map_err(|error| Self::io_failure(folder, &error))?;
        let staged = folder.join(Self::STAGED_NAME);
        let bytes = serde_json::to_vec_pretty(document).map_err(|error| {
            config_failure(&format!(
                "Cannot write the development {}: {error}",
                WebAppManifest::SOURCE_PATH
            ))
        })?;
        fs::write(&staged, bytes).map_err(|error| Self::io_failure(&staged, &error))?;
        fs::rename(&staged, path).map_err(|error| Self::io_failure(path, &error))
    }

    fn io_failure(path: &Path, error: &std::io::Error) -> Failure {
        config_failure(&format!(
            "Cannot prepare the development manifest {}: {error}",
            path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::DevelopmentOrigin;
    use std::fs;
    use tachyon_contracts::NativeTarget;

    fn manifest(root: &std::path::Path) -> serde_json::Value {
        let source = fs::read_to_string(root.join("client/manifest.json")).expect("manifest");
        serde_json::from_str(&source).expect("json")
    }

    #[test]
    fn the_emulator_reaches_the_build_machine_through_localhost_and_the_rest_through_loopback() {
        assert_eq!(
            DevelopmentOrigin::new(NativeTarget::Android, 3000).origin(),
            "http://localhost:3000"
        );
        for target in [
            NativeTarget::Macos,
            NativeTarget::Ios,
            NativeTarget::Linux,
            NativeTarget::Windows,
        ] {
            assert_eq!(
                DevelopmentOrigin::new(target, 4123).origin(),
                "http://127.0.0.1:4123"
            );
        }
        assert_eq!(
            DevelopmentOrigin::new(NativeTarget::Linux, 1).app_origin(),
            "tachyon://app"
        );
    }

    #[test]
    fn a_project_without_a_yon_origin_gets_the_development_server() {
        let root = tempfile::tempdir().expect("root");
        fs::create_dir_all(root.path().join("client")).expect("client");
        fs::write(
            root.path().join("client/manifest.json"),
            r#"{"name":"Dev","tac":{"styles":["/app.css"]}}"#,
        )
        .expect("manifest");
        let note = DevelopmentOrigin::new(NativeTarget::Linux, 3000)
            .declare_in(root.path())
            .expect("declared");
        assert_eq!(note, None);
        let written = manifest(root.path());
        assert_eq!(written["tac"]["yon"]["origin"], "http://127.0.0.1:3000");
        assert_eq!(written["tac"]["styles"][0], "/app.css");
        assert_eq!(written["name"], "Dev");
    }

    #[test]
    fn a_missing_manifest_is_written_with_only_the_development_origin() {
        let root = tempfile::tempdir().expect("root");
        DevelopmentOrigin::new(NativeTarget::Android, 3100)
            .declare_in(root.path())
            .expect("declared");
        assert_eq!(
            manifest(root.path()),
            serde_json::json!({"tac": {"yon": {"origin": "http://localhost:3100"}}})
        );
    }

    #[test]
    fn a_declared_yon_origin_is_replaced_with_a_note() {
        let root = tempfile::tempdir().expect("root");
        fs::create_dir_all(root.path().join("client")).expect("client");
        fs::write(
            root.path().join("client/manifest.json"),
            r#"{"tac":{"yon":{"origin":"https://api.example.com"}}}"#,
        )
        .expect("manifest");
        let origin = DevelopmentOrigin::new(NativeTarget::Ios, 3000);
        let note = origin
            .declare_in(root.path())
            .expect("declared")
            .expect("note");
        assert!(note.contains("http://127.0.0.1:3000"), "{note}");
        assert!(note.contains("https://api.example.com"), "{note}");
        assert_eq!(
            manifest(root.path())["tac"]["yon"]["origin"],
            "http://127.0.0.1:3000"
        );
        assert_eq!(origin.declare_in(root.path()).expect("again"), None);
    }
}
