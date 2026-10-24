use crate::NativeCompanion;
use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::path::Path;
use tachyon_contracts::NativeTarget;

pub struct NativeAppSources;

impl NativeAppSources {
    const CLIENT: &'static str = "client";
    const MANIFEST: &'static str = "manifest.json";
    const MAX_FILES: usize = 65_536;

    #[must_use]
    pub fn fingerprint(project_root: &Path, target: NativeTarget) -> u64 {
        let extensions: Vec<&str> = NativeCompanion::ALL
            .iter()
            .filter(|language| language.supports(target))
            .map(|language| language.model_extension())
            .collect();
        let mut hasher = DefaultHasher::new();
        let mut pending = vec![project_root.join(Self::CLIENT)];
        let mut seen = 0;
        while let Some(current) = pending.pop() {
            let Ok(metadata) = std::fs::symlink_metadata(&current) else {
                continue;
            };
            if metadata.is_dir() {
                pending.extend(Self::children(&current));
                continue;
            }
            seen += 1;
            if seen > Self::MAX_FILES {
                break;
            }
            if Self::builds_the_app(&current, &extensions) {
                current.hash(&mut hasher);
                metadata.len().hash(&mut hasher);
                metadata.modified().ok().hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    fn children(folder: &Path) -> Vec<std::path::PathBuf> {
        let mut children: Vec<_> = std::fs::read_dir(folder)
            .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default();
        children.sort();
        children
    }

    fn builds_the_app(file: &Path, extensions: &[&str]) -> bool {
        let manifest = file.file_name().is_some_and(|name| name == Self::MANIFEST);
        let linked = file
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension));
        [manifest, linked].contains(&true)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::NativeAppSources;
    use std::fs;
    use tachyon_contracts::NativeTarget;

    #[test]
    fn only_the_manifest_and_linked_sources_change_the_app() {
        let root = tempfile::tempdir().expect("root");
        let write = |relative: &str, text: &str| {
            let path = root.path().join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("folders");
            fs::write(path, text).expect("file");
        };
        write("client/manifest.json", "{}");
        write("client/pages/tac.html", "<main></main>");
        write("client/services/Counter.kt", "class Counter");
        let first = NativeAppSources::fingerprint(root.path(), NativeTarget::Linux);
        write("client/pages/tac.html", "<main>changed page</main>");
        write(
            "client/services/Counter.kt",
            "class Counter { val changed = 1 }",
        );
        assert_eq!(
            NativeAppSources::fingerprint(root.path(), NativeTarget::Linux),
            first
        );
        write("client/services/tac.rs", "pub struct Linked;");
        let linked = NativeAppSources::fingerprint(root.path(), NativeTarget::Linux);
        assert_ne!(linked, first);
        write("client/manifest.json", "{\"name\":\"Renamed app\"}");
        assert_ne!(
            NativeAppSources::fingerprint(root.path(), NativeTarget::Linux),
            linked
        );
        let android = NativeAppSources::fingerprint(root.path(), NativeTarget::Android);
        write(
            "client/services/Counter.kt",
            "class Counter { val again = 2 }",
        );
        assert_ne!(
            NativeAppSources::fingerprint(root.path(), NativeTarget::Android),
            android
        );
    }
}
