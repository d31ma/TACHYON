use super::java_code::JavaCode;
use super::java_enclosure::JavaEnclosure;
use super::java_service::{JavaService, JavaServiceDraft};
use crate::Failure;
use crate::behavior::{JavaPageRuntime, JavaRuntimeSections};
use std::collections::BTreeSet;
use std::ops::Range;
use std::path::{Path, PathBuf};

pub(super) struct JavaPageProgram;

struct JavaPageType {
    name: String,
    span: Range<usize>,
}

#[derive(Default)]
struct JavaPageFile {
    imports: Vec<String>,
    types: Vec<(String, String)>,
}

pub(super) struct JavaPageSources {
    pub(super) services: Vec<JavaService>,
    pub(super) page: Option<(PathBuf, String)>,
}

impl JavaPageProgram {
    const FOLDER: &'static str = "tachyon";
    const FILE: &'static str = "TacPage.java";
    const REALM: &'static str = "tac";
    const IMPORT: &'static str = "import ";
    const STATIC_IMPORT: &'static str = "import static ";
    const MODEL_IMPORT: &'static str = "import models.";
    const ALL_NAMES: &'static str = ".*";
    const STATEMENT_END: char = ';';
    const NAME_SEPARATOR: char = '.';
    const LINE_ENDS: [u8; 2] = *b"\n\r";
    const BLANK: u8 = b' ';

    pub(super) fn runtime(drafts: &[JavaServiceDraft]) -> Result<Option<JavaPageRuntime>, Failure> {
        let sections = drafts
            .iter()
            .map(Self::sections)
            .collect::<Result<Vec<_>, _>>()?;
        let joined = sections
            .iter()
            .flatten()
            .try_fold(JavaRuntimeSections::default(), |known, more| {
                known.joined(more)
            });
        let module_id = drafts
            .first()
            .map(|first| crate::behavior::page_module_id(&first.service.path));
        Ok(module_id
            .zip(joined)
            .map(|(module_id, sections)| JavaPageRuntime::new(module_id, sections)))
    }

    pub(super) fn separate(services: Vec<JavaService>) -> JavaPageSources {
        let wrapped = services
            .into_iter()
            .enumerate()
            .map(|(index, service)| JavaService {
                source: JavaEnclosure::wrap(&service.source, &JavaEnclosure::holder(index)),
                ..service
            })
            .collect();
        JavaPageSources {
            services: wrapped,
            page: None,
        }
    }

    pub(super) fn shared(
        services: Vec<JavaService>,
        drafts: &[JavaServiceDraft],
        runtime: &JavaPageRuntime,
    ) -> Option<JavaPageSources> {
        let mut page = JavaPageFile::default();
        let remaining = services
            .iter()
            .zip(drafts)
            .map(|(service, draft)| page.moved(&service.source, &Self::own(draft)))
            .collect::<Option<Vec<_>>>()?;
        let owned: BTreeSet<String> = drafts.iter().flat_map(Self::own).collect();
        let refused = [
            runtime.configured().any(|name| owned.contains(name)),
            page.imports_conflict(),
        ];
        (refused == [false, false]).then(|| page.sources(services, remaining))
    }

    fn sections(draft: &JavaServiceDraft) -> Result<Option<JavaRuntimeSections>, Failure> {
        crate::behavior::java_page_sections(
            &draft.service.path,
            &draft.behavior_source,
            Self::REALM,
        )
    }

    fn own(draft: &JavaServiceDraft) -> BTreeSet<String> {
        JavaCode::new(&draft.service.source)
            .top_level_types()
            .into_iter()
            .map(|declaration| declaration.name)
            .collect()
    }

    fn types(code: &JavaCode) -> Option<(usize, Vec<JavaPageType>)> {
        let declarations = code.top_level_types();
        let opening = declarations
            .first()
            .map_or(0, |first| code.statement_end_before(first.keyword_at));
        let mut start = opening;
        let mut types = Vec::new();
        for declaration in declarations {
            let end = code.body_after(declaration.name_end)?.end + 1;
            types.push(JavaPageType {
                name: declaration.name,
                span: start..end,
            });
            start = end;
        }
        Some((opening, types))
    }

    fn imports(preamble: &str) -> Vec<String> {
        preamble
            .split(Self::STATEMENT_END)
            .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|statement| Self::is_page_import(statement))
            .map(|statement| format!("{statement}{}", Self::STATEMENT_END))
            .collect()
    }

    fn is_page_import(statement: &str) -> bool {
        [
            statement.starts_with(Self::IMPORT),
            statement.starts_with(Self::MODEL_IMPORT),
        ] == [true, false]
    }

    fn named_type(statement: &str) -> Option<&str> {
        let named = [
            statement.starts_with(Self::STATIC_IMPORT),
            statement.ends_with(Self::ALL_NAMES),
        ] == [false, false];
        named
            .then(|| statement.rsplit(Self::NAME_SEPARATOR).next())
            .flatten()
    }

    fn blank(bytes: &mut [u8]) {
        bytes
            .iter_mut()
            .filter(|byte| !Self::LINE_ENDS.contains(byte))
            .for_each(|byte| *byte = Self::BLANK);
    }
}

impl JavaPageFile {
    fn moved(&mut self, source: &str, own: &BTreeSet<String>) -> Option<String> {
        let code = JavaCode::new(source);
        let (opening, types) = JavaPageProgram::types(&code)?;
        self.imports
            .extend(JavaPageProgram::imports(&code.text(&(0..opening))));
        let mut bytes = source.as_bytes().to_vec();
        for moving in types.iter().filter(|found| !own.contains(&found.name)) {
            self.keep(&moving.name, source.get(moving.span.clone())?.trim())?;
            JavaPageProgram::blank(&mut bytes[moving.span.clone()]);
        }
        String::from_utf8(bytes).ok()
    }

    fn keep(&mut self, name: &str, text: &str) -> Option<()> {
        let same = self
            .types
            .iter()
            .find(|(kept, _)| kept == name)
            .map(|(_, kept)| kept == text);
        if let Some(same) = same {
            return same.then_some(());
        }
        self.types.push((name.to_owned(), text.to_owned()));
        Some(())
    }

    fn imports_conflict(&self) -> bool {
        let unique: BTreeSet<&str> = self
            .imports
            .iter()
            .map(|statement| statement.trim_end_matches(JavaPageProgram::STATEMENT_END))
            .collect();
        let named: Vec<&str> = unique
            .iter()
            .filter_map(|statement| JavaPageProgram::named_type(statement))
            .collect();
        let distinct: BTreeSet<&&str> = named.iter().collect();
        distinct.len() != named.len()
    }

    fn sources(&self, services: Vec<JavaService>, remaining: Vec<String>) -> JavaPageSources {
        if self.types.is_empty() {
            return JavaPageProgram::separate(services);
        }
        let wrapped = services
            .into_iter()
            .zip(remaining)
            .enumerate()
            .map(|(index, (service, source))| JavaService {
                source: JavaEnclosure::wrap_on_page(&source, &JavaEnclosure::holder(index)),
                ..service
            })
            .collect();
        JavaPageSources {
            services: wrapped,
            page: Some(self.file()),
        }
    }

    fn file(&self) -> (PathBuf, String) {
        let unique: BTreeSet<&String> = self.imports.iter().collect();
        let imports = unique
            .into_iter()
            .fold(String::new(), |text, statement| text + statement + "\n");
        let types = self
            .types
            .iter()
            .fold(String::new(), |text, (_, declared)| {
                text + "\n" + declared + "\n"
            });
        let path = Path::new(JavaPageProgram::FOLDER).join(JavaPageProgram::FILE);
        (path, JavaEnclosure::page(&format!("{imports}{types}")))
    }
}
