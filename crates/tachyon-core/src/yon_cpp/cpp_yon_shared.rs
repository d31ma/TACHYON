use super::CppYonProvider;
use crate::Failure;
use crate::tac_cpp::{CppFailure, CppFile, CppFolder, CppLowering, CppProgram, CppSource};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) struct CppYonShared {
    name: String,
    path: PathBuf,
    at: usize,
    words: BTreeSet<String>,
    lowered: String,
}

impl CppYonShared {
    const FOLDER: &'static str = "server/shared";
    const CODE: u16 = 1310;
    const MARKED: &'static str = "C++ code in server/shared is one plain class and its #include lines, without a Tachyon marker. Put a class with a marker in its server role folder";
    const LINKED: &'static str = "C++ code in server/shared is read from the project itself, so a symlink there is refused. Copy the file into server/shared";
    const UNSAFE_PATH: &'static str = "A C++ file in server/shared has a name without quotes or control characters. Rename the file";
    const CIRCLE: &'static str = "C++ classes in server/shared use each other in a circle, so none of them can be written first. Move the code that both use into a third class";
    const REPEATED: &'static str = "Another C++ file in server/shared already declares a class with this name. Give each shared class its own name";
    const PROVIDER_NAME: &'static str = "A C++ class in server/shared has the name of a controller, service, delegate, converter, store or HTTP client. Give the shared class another name";
    const USES_PROVIDER: &'static str = "A C++ class in server/shared does not use a controller, service, delegate, converter, store or HTTP client, because they are written after it. Pass the values it needs as arguments";

    pub(crate) fn either(
        path: &Path,
        source: &str,
        other: fn(&Path, &str) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        CppSource::either(path, source, Self::check, other)
    }

    fn check(path: &Path, source: &str) -> Result<(), Failure> {
        Self::read(path, source).map(drop)
    }

    pub(crate) fn read_all(root: &Path) -> Result<Vec<Self>, Failure> {
        let read = CppFolder::files(root, Self::FOLDER)?
            .iter()
            .map(|path| Self::read_in(root, path))
            .collect::<Result<Vec<_>, _>>()?;
        Self::refuse_repeated(&read)?;
        Self::ordered(read)
    }

    pub(crate) fn located(shared: &[Self]) -> String {
        shared
            .iter()
            .map(|class| CppProgram::located(&class.path, &class.lowered))
            .collect()
    }

    pub(crate) fn check_providers(
        shared: &[Self],
        providers: &[CppYonProvider],
    ) -> Result<(), Failure> {
        shared
            .iter()
            .try_for_each(|class| class.check_against(providers))
    }

    fn check_against(&self, providers: &[CppYonProvider]) -> Result<(), Failure> {
        if providers.iter().any(|provider| provider.name == self.name) {
            return Err(self.failure(Self::PROVIDER_NAME));
        }
        if providers
            .iter()
            .any(|provider| self.words.contains(&provider.name))
        {
            return Err(self.failure(Self::USES_PROVIDER));
        }
        Ok(())
    }

    fn failure(&self, message: &str) -> Failure {
        CppFailure::coded(Self::CODE, &self.path, self.at, message)
    }

    fn read_in(root: &Path, path: &Path) -> Result<Self, Failure> {
        Self::refuse_links(root, path)?;
        let (relative, source) = CppFolder::read(root, path)?;
        Self::refuse_unsafe_path(&relative)?;
        Self::read(&relative, &source)
    }

    fn refuse_links(root: &Path, path: &Path) -> Result<(), Failure> {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let linked = relative
            .ancestors()
            .filter(|part| part.starts_with(Self::FOLDER))
            .find(|part| {
                std::fs::symlink_metadata(root.join(part))
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
            });
        linked.map_or(Ok(()), |part| {
            Err(CppFailure::coded(Self::CODE, part, 0, Self::LINKED))
        })
    }

    fn refuse_unsafe_path(relative: &Path) -> Result<(), Failure> {
        let unsafe_name = relative
            .to_string_lossy()
            .chars()
            .any(|character| [character == '"', character.is_control()].contains(&true));
        (!unsafe_name)
            .then_some(())
            .ok_or_else(|| CppFailure::coded(Self::CODE, relative, 0, Self::UNSAFE_PATH))
    }

    fn read(path: &Path, source: &str) -> Result<Self, Failure> {
        let file = CppFile::read(path, source)?;
        let marked = file
            .class
            .every_marker()
            .into_iter()
            .find(|marker| CppLowering::is_tachyon(marker));
        marked.map_or(Ok(()), |marker| {
            Err(CppFailure::coded(Self::CODE, path, marker.at, Self::MARKED))
        })?;
        let words = CppSource::read(source)
            .map_err(|message| CppFailure::unreadable(path, 0, message))?
            .tokens
            .into_iter()
            .filter_map(|token| token.is_word().then_some(token.text))
            .collect();
        Ok(Self {
            name: file.class.name.clone(),
            path: path.to_path_buf(),
            at: file.class.name_at,
            words,
            lowered: CppLowering::model(&file, source),
        })
    }

    fn refuse_repeated(read: &[Self]) -> Result<(), Failure> {
        let repeated = read
            .iter()
            .enumerate()
            .find(|(index, class)| read[..*index].iter().any(|seen| seen.name == class.name));
        repeated.map_or(Ok(()), |(_, class)| Err(class.failure(Self::REPEATED)))
    }

    fn ordered(mut waiting: Vec<Self>) -> Result<Vec<Self>, Failure> {
        let mut ordered = Vec::new();
        while !waiting.is_empty() {
            let ready = waiting
                .iter()
                .position(|shared| !shared.uses_any(&waiting))
                .ok_or_else(|| Self::circle(&waiting))?;
            ordered.push(waiting.remove(ready));
        }
        Ok(ordered)
    }

    fn circle(waiting: &[Self]) -> Failure {
        let mut seen: Vec<&str> = Vec::new();
        let mut current = &waiting[0];
        while !seen.contains(&current.name.as_str()) {
            seen.push(&current.name);
            current = waiting
                .iter()
                .find(|other| current.uses(other))
                .unwrap_or(current);
        }
        current.failure(Self::CIRCLE)
    }

    fn uses_any(&self, waiting: &[Self]) -> bool {
        waiting.iter().any(|other| self.uses(other))
    }

    fn uses(&self, other: &Self) -> bool {
        [other.name != self.name, self.words.contains(&other.name)] == [true, true]
    }
}
