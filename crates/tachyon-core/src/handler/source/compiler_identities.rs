use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub(super) struct CompilerIdentities;

impl CompilerIdentities {
    const SETTINGS: [&'static str; 2] = ["PATH", "JAVA_HOME"];

    pub(super) fn remembered(program: &str, probe: impl FnOnce() -> String) -> String {
        let key = Self::key(program);
        let known = Self::known_identity(&key);
        known.unwrap_or_else(|| Self::remember(key, probe()))
    }

    fn key(program: &str) -> String {
        let mut key = String::from(program);
        for setting in Self::SETTINGS {
            key.push('\0');
            key.push_str(
                &std::env::var_os(setting)
                    .unwrap_or_default()
                    .to_string_lossy(),
            );
        }
        key
    }

    fn known_identity(key: &str) -> Option<String> {
        Self::known().lock().ok()?.get(key).cloned()
    }

    fn remember(key: String, identity: String) -> String {
        if let Ok(mut map) = Self::known().lock() {
            map.insert(key, identity.clone());
        }
        identity
    }

    fn known() -> &'static Mutex<HashMap<String, String>> {
        static KNOWN: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
        KNOWN.get_or_init(|| Mutex::new(HashMap::new()))
    }
}
