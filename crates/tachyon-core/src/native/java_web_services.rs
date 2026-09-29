use crate::Failure;
use java_browser_runtime::JavaBrowserRuntime;
use java_class_reader::JavaClassReader;
use java_enclosure::JavaEnclosure;
use java_event_members::JavaEventMembers;
use java_models::JavaModels;
use java_service::JavaService;
use java_service_table::JavaServiceTable;
use java_value_type::JavaValueType;
use java_wasm_program::JavaWasmProgram;
use std::path::{Path, PathBuf};

mod java_browser_runtime;
mod java_class_reader;
mod java_code;
mod java_declaration;
mod java_enclosure;
mod java_event_members;
mod java_member;
mod java_member_kind;
mod java_model;
mod java_model_kind;
mod java_model_reader;
mod java_model_table;
mod java_models;
mod java_service;
mod java_service_table;
mod java_type_declaration;
mod java_value_type;
mod java_wasm_program;
mod tea_vm_toolchain;

pub(crate) struct JavaWebServices;

impl JavaWebServices {
    const PRELUDE: [(&'static str, &'static str); 20] = [
        (
            "TacServices.java",
            include_str!("java_web_services/prelude/TacServices.java"),
        ),
        (
            "TacBridge.java",
            include_str!("java_web_services/prelude/TacBridge.java"),
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
    ];
    const PRELUDE_FOLDER: &'static str = "tachyon";

    pub(crate) fn teavm_installed() -> bool {
        JavaWasmProgram::is_installed()
    }

    pub(crate) async fn build(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
        profile: crate::BuildProfile,
    ) -> Result<(Vec<(String, Vec<u8>)>, Option<String>), Failure> {
        let models = JavaModels::read(snapshot_root)?;
        let values = models.values();
        let read = Self::services(snapshot_root, services, &values)?;
        let mut sources: Vec<(PathBuf, String)> = read
            .iter()
            .map(|service| (service.path.clone(), service.source.clone()))
            .collect();
        sources.extend(models.sources(&values)?);
        sources.extend(Self::prelude(&read, &values));
        let program = JavaWasmProgram::compile(&sources, profile).await?;
        let note = program.note(profile);
        Ok((program.published(), note))
    }

    fn services(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
        values: &JavaValueType,
    ) -> Result<Vec<JavaService>, Failure> {
        let mut read = Vec::new();
        for (index, (instance, class, path)) in services.iter().enumerate() {
            let relative = path.strip_prefix(snapshot_root).unwrap_or(path);
            let authored = Self::read(path)?;
            let service = JavaService {
                instance: instance.to_owned(),
                class: class.to_owned(),
                members: Vec::new(),
                required: Vec::new(),
                path: relative.to_path_buf(),
                source: authored,
            };
            read.push(Self::service(snapshot_root, service, index, values)?);
        }
        Ok(read)
    }

    fn service(
        snapshot_root: &Path,
        authored: JavaService,
        index: usize,
        values: &JavaValueType,
    ) -> Result<JavaService, Failure> {
        let relative = authored.path.as_path();
        crate::annotation_layout::NativeWebApis::refuse(relative, &authored.source)?;
        let roles = crate::behavior::ServiceRoles::read(relative, &authored.source)?;
        let assembled =
            crate::annotation_modules::assemble(snapshot_root, relative, &roles.source, "tac")?;
        let source = assembled.as_deref().unwrap_or(&roles.source);
        let mut events = crate::events::extract(relative, source, "tac")?;
        if let Some(module) = events.as_mut() {
            crate::events::describe(module, relative, snapshot_root, "client")?;
        }
        let lowered = crate::behavior::lower(relative, source, "tac")?;
        let source = lowered.map_or_else(
            || source.to_owned(),
            |text| JavaBrowserRuntime::adapt(&text),
        );
        let shown = relative.to_string_lossy();
        let reader = JavaClassReader::new(&shown, &source);
        let events_of_class = JavaEventMembers::new(relative, &authored.class, values);
        let mut members = reader.members(&authored.class, values)?;
        members.extend(events_of_class.listeners(events.as_ref())?);
        let compiled = JavaEventMembers::with_publishers(&reader.without_markers(&source));
        Ok(JavaService {
            members,
            source: JavaEnclosure::wrap(&compiled, &JavaEnclosure::holder(index)),
            required: roles.required,
            ..authored
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

    fn prelude(services: &[JavaService], values: &JavaValueType) -> Vec<(PathBuf, String)> {
        let folder = Path::new(Self::PRELUDE_FOLDER);
        let (table_name, table) = JavaServiceTable::new(values).file(services);
        let mut files: Vec<(PathBuf, String)> = Self::PRELUDE
            .iter()
            .map(|(name, text)| (folder.join(name), String::from(*text)))
            .collect();
        files.push((folder.join(table_name), table));
        files
    }
}
