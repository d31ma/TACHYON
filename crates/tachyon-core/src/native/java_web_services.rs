use crate::Failure;
use crate::behavior::JavaPageRuntime;
use java_browser_runtime::JavaBrowserRuntime;
use java_class_reader::JavaClassReader;
use java_event_members::JavaEventMembers;
use java_held_streams::JavaHeldStreams;
use java_models::JavaModels;
use java_page_program::JavaPageProgram;
use java_page_sources::JavaPageSources;
use java_service::JavaService;
use java_service_draft::JavaServiceDraft;
use java_service_needs::JavaServiceNeeds;
use java_service_table::JavaServiceTable;
use java_value_type::JavaValueType;
use java_wasm_program::JavaWasmProgram;
use java_web_apis::JavaWebApis;
use std::path::{Path, PathBuf};
use tea_vm_toolchain::TeaVmToolchain;

mod java_browser_runtime;
mod java_class_reader;
mod java_code;
mod java_composed_types;
mod java_constructor_provider;
mod java_declaration;
mod java_enclosure;
mod java_event_members;
mod java_held_streams;
mod java_member;
mod java_member_kind;
mod java_model;
mod java_model_kind;
mod java_model_reader;
mod java_model_table;
mod java_models;
mod java_need_owner;
mod java_page_file;
mod java_page_program;
mod java_page_sources;
mod java_page_type;
mod java_preference_constructor;
mod java_service;
mod java_service_draft;
mod java_service_needs;
mod java_service_table;
mod java_test_contracts;
mod java_type_declaration;
mod java_type_members;
mod java_type_models;
mod java_value_type;
mod java_wasm_program;
mod java_web_apis;
mod tea_vm_runtime;
mod tea_vm_toolchain;

#[cfg(test)]
mod java_browser_http_tests;
#[cfg(test)]
mod java_page_program_tests;
#[cfg(test)]
mod java_service_needs_tests;
#[cfg(test)]
mod preference_strings_tests;

pub(crate) struct JavaWebServices;

impl JavaWebServices {
    const PRELUDE: [(&'static str, &'static str); 30] = [
        (
            "TacNode.java",
            include_str!("java_web_services/prelude/TacNode.java"),
        ),
        (
            "TacNeeds.java",
            include_str!("java_web_services/prelude/TacNeeds.java"),
        ),
        (
            "TacPreferenceScope.java",
            include_str!("java_web_services/prelude/TacPreferenceScope.java"),
        ),
        (
            "TacServices.java",
            include_str!("java_web_services/prelude/TacServices.java"),
        ),
        (
            "TacBridge.java",
            include_str!("java_web_services/prelude/TacBridge.java"),
        ),
        (
            "TacFailure.java",
            include_str!("java_web_services/prelude/TacFailure.java"),
        ),
        (
            "TacTask.java",
            include_str!("java_web_services/prelude/TacTask.java"),
        ),
        (
            "TacTasks.java",
            include_str!("java_web_services/prelude/TacTasks.java"),
        ),
        (
            "TacHttpTickets.java",
            include_str!("java_web_services/prelude/TacHttpTickets.java"),
        ),
        (
            "TacActivation.java",
            include_str!("java_web_services/prelude/TacActivation.java"),
        ),
        (
            "TacField.java",
            include_str!("java_web_services/prelude/TacField.java"),
        ),
        (
            "TacMethod.java",
            include_str!("java_web_services/prelude/TacMethod.java"),
        ),
        (
            "TacHook.java",
            include_str!("java_web_services/prelude/TacHook.java"),
        ),
        (
            "TacListener.java",
            include_str!("java_web_services/prelude/TacListener.java"),
        ),
        (
            "TacCall.java",
            include_str!("java_web_services/prelude/TacCall.java"),
        ),
        (
            "TacValues.java",
            include_str!("java_web_services/prelude/TacValues.java"),
        ),
        (
            "TacJsonReader.java",
            include_str!("java_web_services/prelude/TacJsonReader.java"),
        ),
        (
            "TacJsonWriter.java",
            include_str!("java_web_services/prelude/TacJsonWriter.java"),
        ),
        (
            "TacEvent.java",
            include_str!("java_web_services/prelude/TacEvent.java"),
        ),
        (
            "TacEvents.java",
            include_str!("java_web_services/prelude/TacEvents.java"),
        ),
        (
            "TacHost.java",
            include_str!("java_web_services/prelude/TacHost.java"),
        ),
        (
            "TacStorage.java",
            include_str!("java_web_services/prelude/TacStorage.java"),
        ),
        (
            "ApplicationEventPublisher.java",
            include_str!("java_web_services/prelude/ApplicationEventPublisher.java"),
        ),
        (
            "TacPermits.java",
            include_str!("java_web_services/prelude/TacPermits.java"),
        ),
        (
            "TacJoiner.java",
            include_str!("java_web_services/prelude/TacJoiner.java"),
        ),
        (
            "TacPause.java",
            include_str!("java_web_services/prelude/TacPause.java"),
        ),
        (
            "TacMaps.java",
            include_str!("java_web_services/prelude/TacMaps.java"),
        ),
        (
            "TacText.java",
            include_str!("java_web_services/prelude/TacText.java"),
        ),
        (
            "TacStream.java",
            include_str!("java_web_services/prelude/TacStream.java"),
        ),
        (
            "TacSocket.java",
            include_str!("java_web_services/prelude/TacSocket.java"),
        ),
    ];
    const PRELUDE_FOLDER: &'static str = "tachyon";
    const TYPED_ANSWER: &'static str = "Behaviors.answer(";
    const VALUE_READERS: &'static str = include_str!("java_web_services/prelude/TacValues.java");

    pub(crate) fn teavm_installed() -> bool {
        JavaWasmProgram::is_installed()
    }

    pub(crate) fn teavm_home() -> PathBuf {
        TeaVmToolchain::home()
    }

    pub(crate) fn managed_teavm_folder() -> PathBuf {
        TeaVmToolchain::managed_folder()
    }

    pub(crate) async fn build(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
        profile: crate::BuildProfile,
    ) -> Result<(Vec<(String, Vec<u8>)>, Option<String>), Failure> {
        let sources = Self::sources(snapshot_root, services)?;
        let program = JavaWasmProgram::compile(&sources, profile).await?;
        let note = program.note(profile);
        Ok((program.published(), note))
    }

    fn sources(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
    ) -> Result<Vec<(PathBuf, String)>, Failure> {
        let authored = Self::authored(services)?;
        let all_authored = authored.concat();
        let models = JavaModels::read(snapshot_root, &all_authored)?;
        let values = models.values();
        let needs = JavaServiceNeeds::read(services, &authored)?;
        let drafts = Self::drafts(snapshot_root, services, &authored, (&models, &needs))?;
        let runtime = JavaPageProgram::runtime(&drafts)?;
        let page = (&values, &models, &needs);
        let built = Self::built(&drafts, page, runtime)?;
        let read = built.services;
        let shared = JavaServiceNeeds::shared(&read, &authored)?;
        let mut sources: Vec<(PathBuf, String)> = read
            .iter()
            .map(|service| (service.path.clone(), service.source.clone()))
            .collect();
        sources.extend(built.page);
        sources.extend(models.sources(&values)?);
        sources.extend(Self::prelude(&read, &values, &shared));
        sources.extend(JavaWebApis::clients(
            Path::new(Self::PRELUDE_FOLDER),
            &all_authored,
        ));
        Ok(sources)
    }

    fn built(
        drafts: &[JavaServiceDraft],
        page: (&JavaValueType, &JavaModels, &JavaServiceNeeds),
        runtime: Option<JavaPageRuntime>,
    ) -> Result<JavaPageSources, Failure> {
        let shared = runtime
            .map(|runtime| Self::shared(drafts, page, &runtime))
            .transpose()?
            .flatten();
        shared.map_or_else(
            || Self::services(drafts, page, None).map(JavaPageProgram::separate),
            Ok,
        )
    }

    fn shared(
        drafts: &[JavaServiceDraft],
        page: (&JavaValueType, &JavaModels, &JavaServiceNeeds),
        runtime: &JavaPageRuntime,
    ) -> Result<Option<JavaPageSources>, Failure> {
        let finished = Self::services(drafts, page, Some(runtime))?;
        Ok(JavaPageProgram::shared(finished, drafts, runtime))
    }

    pub(crate) fn test_answer_readers(project: &Path, lowered: &str) -> Result<String, Failure> {
        let needed = lowered.contains(Self::TYPED_ANSWER);
        needed
            .then(|| Self::answer_readers(project))
            .transpose()
            .map(Option::unwrap_or_default)
    }

    fn answer_readers(project: &Path) -> Result<String, Failure> {
        let models = JavaModels::read(project, "")?;
        let table = models.table(&models.values());
        Ok(format!("{}\n{table}", Self::VALUE_READERS))
    }

    pub(crate) fn answer_reading(declared: &str, value: &str) -> Option<String> {
        JavaValueType::for_answer(declared).reading(declared, value)
    }

    pub(crate) fn composed_types(
        services: &[(PathBuf, String)],
        models: &[(PathBuf, String)],
    ) -> Result<String, Failure> {
        java_composed_types::JavaComposedTypes::render(services, models)
    }

    pub(crate) fn test_provider_contracts(source: &str) -> String {
        java_test_contracts::JavaTestContracts::read(source)
    }

    fn authored(services: &[(String, String, PathBuf)]) -> Result<Vec<String>, Failure> {
        services
            .iter()
            .map(|(_, _, path)| Self::read(path))
            .collect()
    }

    fn drafts(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
        texts: &[String],
        (models, needs): (&JavaModels, &JavaServiceNeeds),
    ) -> Result<Vec<JavaServiceDraft>, Failure> {
        let mut drafts = Vec::new();
        for ((instance, class, path), authored) in services.iter().zip(texts) {
            let relative = path.strip_prefix(snapshot_root).unwrap_or(path);
            let service = JavaService {
                instance: instance.to_owned(),
                class: class.to_owned(),
                members: Vec::new(),
                required: Vec::new(),
                providers: Vec::new(),
                path: relative.to_path_buf(),
                source: authored.clone(),
            };
            drafts.push(Self::draft(snapshot_root, service, (models, needs))?);
        }
        Ok(drafts)
    }

    fn draft(
        snapshot_root: &Path,
        authored: JavaService,
        (models, needs): (&JavaModels, &JavaServiceNeeds),
    ) -> Result<JavaServiceDraft, Failure> {
        let relative = authored.path.as_path();
        let roles = crate::behavior::ServiceRoles::read(relative, &authored.source)?;
        let assembled =
            crate::annotation_modules::assemble(snapshot_root, relative, &roles.source, "tac")?;
        let source = assembled.as_deref().unwrap_or(&roles.source);
        let mut events = crate::events::extract(relative, source, "tac")?;
        if let Some(module) = events.as_mut() {
            crate::events::describe(module, relative, snapshot_root, "client")?;
        }
        let held =
            JavaHeldStreams::new(relative, &authored.class).members(events.as_ref(), source)?;
        let behavior_source = models.behavior_source(source, &authored.class)?;
        needs.owner(
            &authored.instance,
            relative,
            (&authored.source, &behavior_source, &authored.class),
        )?;
        Ok(JavaServiceDraft {
            required: roles.required,
            events,
            held,
            behavior_source,
            service: authored,
        })
    }

    fn services(
        drafts: &[JavaServiceDraft],
        page: (&JavaValueType, &JavaModels, &JavaServiceNeeds),
        runtime: Option<&JavaPageRuntime>,
    ) -> Result<Vec<JavaService>, Failure> {
        drafts
            .iter()
            .map(|draft| Self::service(draft, page, runtime))
            .collect()
    }

    fn service(
        draft: &JavaServiceDraft,
        (values, models, needs): (&JavaValueType, &JavaModels, &JavaServiceNeeds),
        runtime: Option<&JavaPageRuntime>,
    ) -> Result<JavaService, Failure> {
        let authored = &draft.service;
        let relative = authored.path.as_path();
        let behavior_source = &draft.behavior_source;
        let owner = needs.owner(
            &authored.instance,
            relative,
            (&authored.source, behavior_source, &authored.class),
        )?;
        let lowered = match runtime {
            Some(runtime) => runtime.lowered(relative, behavior_source, "tac")?,
            None => crate::behavior::lower(relative, behavior_source, "tac")?,
        };
        let source = lowered.map_or_else(
            || behavior_source.clone(),
            |text| JavaBrowserRuntime::adapt(&text),
        );
        let source = models.without_declarations(relative, &source)?;
        let providers = java_preference_constructor::JavaPreferenceConstructor::read(
            relative,
            &source,
            &authored.class,
            &owner,
        )?;
        let needed: Vec<String> = providers
            .iter()
            .filter(|provider| provider.needed().is_some())
            .map(|provider| provider.name().to_owned())
            .collect();
        let shown = relative.to_string_lossy();
        let reader = JavaClassReader::new(&shown, &source);
        let events_of_class = JavaEventMembers::new(relative, &authored.class, values);
        let mut members = reader.members_without_fields(&authored.class, values, &needed)?;
        members.extend(events_of_class.listeners(draft.events.as_ref())?);
        members.extend(draft.held.iter().cloned());
        let compiled =
            JavaEventMembers::with_publishers(&reader.without_markers(&source), &providers);
        let compiled = needs.qualified(&compiled, &authored.class, &providers);
        Ok(JavaService {
            instance: authored.instance.clone(),
            class: authored.class.clone(),
            members,
            providers,
            source: compiled,
            required: draft.required.clone(),
            path: authored.path.clone(),
        })
    }

    fn read(path: &Path) -> Result<String, Failure> {
        let text = std::fs::read_to_string(path).map_err(|error| {
            super::host::native_tool_failure(
                1605,
                &format!("Cannot read the Java web service: {error}"),
            )
        })?;
        Ok(crate::without_bom(&text).to_owned())
    }

    fn prelude(
        services: &[JavaService],
        values: &JavaValueType,
        shared: &std::collections::BTreeSet<String>,
    ) -> Vec<(PathBuf, String)> {
        let folder = Path::new(Self::PRELUDE_FOLDER);
        let (table_name, table) = JavaServiceTable::new(values).file(services, shared);
        let mut files: Vec<(PathBuf, String)> = Self::PRELUDE
            .iter()
            .map(|(name, text)| (folder.join(name), String::from(*text)))
            .collect();
        files.push((folder.join(table_name), table));
        files
    }
}
