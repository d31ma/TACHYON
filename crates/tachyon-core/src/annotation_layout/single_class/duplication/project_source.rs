use super::super::rule_switch::RuleSwitch;
use std::path::Path;

pub(in super::super) struct ProjectSource {
    pub(in super::super) path: String,
    pub(in super::super) text: String,
}

impl ProjectSource {
    const CLIENT: &'static str = "client";
    const SERVER: &'static str = "server";

    pub(in super::super) fn new(path: &Path, text: String) -> Self {
        let view = crate::fsharp_source::FsharpView::of(path, &text);
        let path = view.as_ref().map_or(path, |view| view.path());
        Self {
            path: path.to_string_lossy().replace('\\', "/"),
            text: view.map_or(text, |view| view.text().to_owned()),
        }
    }

    pub(in super::super) fn switched_on<'p>(
        files: impl Iterator<Item = (&'p Path, &'p [u8])>,
        switch: RuleSwitch,
    ) -> Vec<Self> {
        files
            .filter(|(path, _)| switch.covers(path))
            .filter_map(|(path, bytes)| Self::from_bytes(path, bytes))
            .collect()
    }

    pub(in super::super) fn realm(&self) -> &'static str {
        let client = self.path.starts_with("client/");
        [Self::SERVER, Self::CLIENT][usize::from(client)]
    }

    pub(in super::super) fn extension(&self) -> &str {
        Path::new(&self.path)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
    }

    pub(in super::super) fn language(&self) -> String {
        format!("{}/{}", self.realm(), self.extension())
    }

    pub(in super::super) fn from_bytes(path: &Path, bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        Some(Self::new(path, text.to_owned()))
    }
}
