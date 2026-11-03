use super::CppYonController;
use crate::Failure;
use crate::tac_cpp::{CppAttribute, CppClass, CppFailure};
use crate::yon::plan::Scope;
use serde_json::Value;
use std::path::Path;

pub(crate) struct CppYonScope;

impl CppYonScope {
    const MARKER: &'static str = "Lifetime";
    const SCOPES: [(&'static str, Scope); 3] = [
        ("singleton", Scope::Singleton),
        ("request", Scope::Request),
        ("transient", Scope::Transient),
    ];
    const FORM: &'static str = "A Yon [[Lifetime]] takes one quoted scope, \"singleton\", \"request\" or \"transient\", for example [[Lifetime(\"transient\")]]. A singleton is shared, a request-scoped object lives for one request, and a transient object is new for each field that needs it";

    pub(crate) fn of(path: &Path, class: &CppClass, unwritten: Scope) -> Result<Scope, Failure> {
        let marker = class
            .markers
            .iter()
            .find(|marker| marker.name == Self::MARKER);
        marker.map_or(Ok(unwritten), |marker| Self::written(path, marker))
    }

    fn written(path: &Path, marker: &CppAttribute) -> Result<Scope, Failure> {
        let value: Value = serde_json::from_str(&marker.arguments).unwrap_or(Value::Null);
        let word = Self::word(&value);
        Self::SCOPES
            .iter()
            .find(|(name, _)| Some(*name) == word)
            .map(|(_, scope)| *scope)
            .ok_or_else(|| CppFailure::coded(CppYonController::CODE, path, marker.at, Self::FORM))
    }

    fn word(value: &Value) -> Option<&str> {
        let object = value.get("value").and_then(Value::as_str);
        value.as_str().or(object)
    }
}
