use super::java_code::JavaCode;
use super::java_enclosure::JavaEnclosure;
use super::java_page_file::JavaPageFile;
use super::java_page_sources::JavaPageSources;
use super::java_service::JavaService;
use super::java_service_draft::JavaServiceDraft;
use crate::Failure;
use crate::behavior::{JavaPageRuntime, JavaRuntimeSections};
use std::collections::BTreeSet;
use std::path::PathBuf;

pub(super) struct JavaPageProgram;

impl JavaPageProgram {
    const REALM: &'static str = "tac";

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
        let first = drafts.first().map(|first| first.service.path.as_path());
        Ok(first
            .zip(joined)
            .map(|(first, sections)| JavaPageRuntime::new(first, sections)))
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
        (refused == [false, false]).then(|| Self::on_page(services, remaining, page.file()))
    }

    fn on_page(
        services: Vec<JavaService>,
        remaining: Vec<String>,
        page: Option<(PathBuf, String)>,
    ) -> JavaPageSources {
        let Some(page) = page else {
            return Self::separate(services);
        };
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
            page: Some(page),
        }
    }

    fn sections(draft: &JavaServiceDraft) -> Result<Option<JavaRuntimeSections>, Failure> {
        JavaRuntimeSections::read(&draft.service.path, &draft.behavior_source, Self::REALM)
    }

    fn own(draft: &JavaServiceDraft) -> BTreeSet<String> {
        JavaCode::new(&draft.service.source)
            .top_level_types()
            .into_iter()
            .map(|declaration| declaration.name)
            .collect()
    }
}
