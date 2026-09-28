use cap_std::ambient_authority;
use cap_std::fs::Dir;
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub(super) struct ProjectWords {
    tac: Vec<String>,
    yon: Vec<String>,
}

impl ProjectWords {
    const MANIFEST: &'static str = "client/manifest.json";
    const APPLICATION: &'static str = "server/application.json";
    const TAC_POINTER: &'static str = "/tac/names/words";
    const YON_POINTER: &'static str = "/names/words";
    const CLIENT_PREFIX: &'static str = "client/";

    pub(super) fn read(project: &Dir) -> Self {
        Self {
            tac: Self::listed(project, Self::MANIFEST, Self::TAC_POINTER),
            yon: Self::listed(project, Self::APPLICATION, Self::YON_POINTER),
        }
    }

    pub(super) fn read_folder(root: &Path) -> Self {
        Dir::open_ambient_dir(root, ambient_authority())
            .map(|project| Self::read(&project))
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub(super) fn of(tac: &[&str], yon: &[&str]) -> Self {
        Self {
            tac: tac.iter().map(|word| (*word).to_owned()).collect(),
            yon: yon.iter().map(|word| (*word).to_owned()).collect(),
        }
    }

    pub(super) fn for_path(&self, path: &str) -> &[String] {
        let client = path.starts_with(Self::CLIENT_PREFIX);
        [&self.yon, &self.tac][usize::from(client)]
    }

    fn listed(project: &Dir, file: &str, pointer: &str) -> Vec<String> {
        Self::stored(project, file, pointer).unwrap_or_default()
    }

    fn stored(project: &Dir, file: &str, pointer: &str) -> Option<Vec<String>> {
        let bytes = project.read(file).ok()?;
        let document: Value = serde_json::from_slice(&bytes).ok()?;
        let words = document.pointer(pointer)?.as_array()?;
        Some(
            words
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_ascii_lowercase)
                .collect(),
        )
    }
}
