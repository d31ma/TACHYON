use crate::Failure;
use java_class_reader::JavaClassReader;
use java_service::JavaService;
use java_service_table::JavaServiceTable;
use java_wasm_program::JavaWasmProgram;
use std::path::{Path, PathBuf};

mod java_class_reader;
mod java_declaration;
mod java_member;
mod java_member_kind;
mod java_service;
mod java_service_table;
mod java_value_type;
mod java_wasm_program;
mod tea_vm_toolchain;

pub(crate) struct JavaWebServices;

impl JavaWebServices {
    const PRELUDE: [(&'static str, &'static str); 10] = [
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
        let read = Self::services(snapshot_root, services)?;
        let mut sources: Vec<(PathBuf, String)> = read
            .iter()
            .map(|service| (service.path.clone(), service.source.clone()))
            .collect();
        sources.extend(Self::prelude(&read));
        let program = JavaWasmProgram::compile(&sources, profile).await?;
        let note = program.note(profile);
        Ok((program.published(), note))
    }

    fn services(
        snapshot_root: &Path,
        services: &[(String, String, PathBuf)],
    ) -> Result<Vec<JavaService>, Failure> {
        let mut read = Vec::new();
        for (instance, class, path) in services {
            let relative = path.strip_prefix(snapshot_root).unwrap_or(path);
            read.push(Self::service(relative, path, (instance, class))?);
        }
        Ok(read)
    }

    fn service(
        relative: &Path,
        path: &Path,
        (instance, class): (&str, &str),
    ) -> Result<JavaService, Failure> {
        let authored = Self::read(path)?;
        crate::annotation_layout::NativeWebApis::refuse(relative, &authored)?;
        let roles = crate::behavior::ServiceRoles::read(relative, &authored)?;
        let shown = relative.to_string_lossy();
        let reader = JavaClassReader::new(&shown, &roles.source);
        Ok(JavaService {
            instance: instance.to_owned(),
            class: class.to_owned(),
            members: reader.members(class)?,
            source: reader.without_markers(&roles.source),
            required: roles.required,
            path: relative.to_path_buf(),
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

    fn prelude(services: &[JavaService]) -> Vec<(PathBuf, String)> {
        let folder = Path::new(Self::PRELUDE_FOLDER);
        let (table_name, table) = JavaServiceTable::file(services);
        let mut files: Vec<(PathBuf, String)> = Self::PRELUDE
            .iter()
            .map(|(name, text)| (folder.join(name), String::from(*text)))
            .collect();
        files.push((folder.join(table_name), table));
        files
    }
}
