use super::ModelFile;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::Path;

pub(super) struct ModelSelection {
    names: Vec<BTreeSet<String>>,
    chosen: Vec<bool>,
    words: BTreeSet<String>,
}

impl ModelSelection {
    const NAME_MARKS: [char; 2] = ['_', '$'];

    pub(super) fn used(
        extension: &str,
        files: &[ModelFile],
        programs: [&str; 2],
    ) -> Vec<ModelFile> {
        let mut selection = Self {
            names: files
                .iter()
                .map(|file| Self::declared(extension, file))
                .collect(),
            chosen: vec![false; files.len()],
            words: programs.into_iter().flat_map(Self::words).collect(),
        };
        selection.grow(files);
        files
            .iter()
            .zip(selection.chosen)
            .filter(|(_, chosen)| *chosen)
            .map(|(file, _)| file.clone())
            .collect()
    }

    fn declared(extension: &str, file: &ModelFile) -> BTreeSet<String> {
        let path = Path::new(&file.portable);
        let mut names: BTreeSet<String> =
            crate::route_types::ModelTable::declared_names(extension, &file.plain)
                .into_iter()
                .collect();
        names.extend(
            crate::annotation_layout::declaration_names(path, &file.plain).unwrap_or_default(),
        );
        names.extend(path.file_stem().and_then(OsStr::to_str).map(str::to_owned));
        names
    }

    fn grow(&mut self, files: &[ModelFile]) {
        while self.choose_next(files) {}
    }

    fn choose_next(&mut self, files: &[ModelFile]) -> bool {
        let next = (0..files.len()).find(|index| self.wanted(*index));
        next.inspect(|index| self.choose(*index, &files[*index].plain))
            .is_some()
    }

    fn wanted(&self, index: usize) -> bool {
        let named = !self.names[index].is_disjoint(&self.words);
        [self.chosen[index], named] == [false, true]
    }

    fn choose(&mut self, index: usize, plain: &str) {
        self.chosen[index] = true;
        self.words.extend(Self::words(plain));
    }

    fn words(text: &str) -> BTreeSet<String> {
        text.split(|character: char| {
            ![
                character.is_alphanumeric(),
                Self::NAME_MARKS.contains(&character),
            ]
            .contains(&true)
        })
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
    }
}
