use super::fsharp_behaviors_project::FsharpBehaviorsProject;
use crate::Failure;
use crate::fsharp_source::FsharpSource;
use crate::model_sources::ModelSources;
use crate::route_types::{ModelTable, WebEventSources};
use std::path::Path;

pub(super) struct FsharpWebProject;

impl FsharpWebProject {
    const FOLDER: &'static str = "FsharpServices";
    const CONTEXT_CLASS: &'static str = "internal partial class TacModelContext :";
    const SERIALIZABLE: &'static str = "[System.Text.Json.Serialization.JsonSerializable";
    const PROJECT_END: &'static str = "</Project>";
    const REFERENCE: &'static str = "  <ItemGroup><ProjectReference Include=\"FsharpServices/FsharpServices.fsproj\" /></ItemGroup>\n";
    const PROJECT: &'static str = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net9.0</TargetFramework>\n    <OutputType>Library</OutputType>\n    <AssemblyName>FsharpServices</AssemblyName>\n    <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n    <SatelliteResourceLanguages>en</SatelliteResourceLanguages>\n  </PropertyGroup>\n  <ItemGroup><Compile Include=\"WebEvents.fs\" /><Compile Include=\"Services.fs\" /></ItemGroup>\n</Project>\n";

    pub(super) fn with_model_context(
        csharp: &str,
        snapshot_root: &Path,
        fsharp: &str,
    ) -> Result<String, Failure> {
        let wanted = (!fsharp.is_empty()).then(|| Self::model_context(csharp, snapshot_root));
        wanted.unwrap_or_else(|| Ok(csharp.to_owned()))
    }

    fn model_context(csharp: &str, snapshot_root: &Path) -> Result<String, Failure> {
        let models = ModelTable::read_realm(snapshot_root, "client", FsharpSource::EXTENSION)
            .map_err(|message| super::host::native_tool_failure(1605, &message))?;
        let attributes = models
            .record_names()
            .iter()
            .map(|name| format!("{}(typeof(Models.{name}))]\n", Self::SERIALIZABLE))
            .collect::<Vec<_>>()
            .concat();
        let context = format!("{attributes}{}", Self::CONTEXT_CLASS);
        Ok(csharp.replacen(Self::CONTEXT_CLASS, &context, 1))
    }

    pub(super) fn referenced(project: &str, fsharp: &str) -> String {
        let reference = format!("{}{}", Self::REFERENCE, Self::PROJECT_END);
        let wanted = (!fsharp.is_empty()).then(|| project.replace(Self::PROJECT_END, &reference));
        wanted.unwrap_or_else(|| project.to_owned())
    }

    fn wanted(files: Vec<(String, String)>, fsharp: &str) -> Vec<(String, String)> {
        if fsharp.is_empty() {
            return Vec::new();
        }
        files
    }

    pub(super) fn files(
        snapshot_root: &Path,
        fsharp: &str,
        runtime: &str,
    ) -> Result<Vec<(String, String)>, Failure> {
        let models = ModelSources::read(snapshot_root, "client", FsharpSource::EXTENSION)?;
        let services = format!(
            "{}{fsharp}",
            FsharpSource::models_file(&models.separate_files())
        );
        let mut files = vec![
            (
                format!("{}/FsharpServices.fsproj", Self::FOLDER),
                Self::services_project(),
            ),
            (format!("{}/Services.fs", Self::FOLDER), services),
            (
                format!("{}/WebEvents.fs", Self::FOLDER),
                WebEventSources::fsharp(),
            ),
        ];
        files.extend(FsharpBehaviorsProject::files(runtime));
        Ok(Self::wanted(files, fsharp))
    }

    fn services_project() -> String {
        let reference = format!("{}{}", FsharpBehaviorsProject::REFERENCE, Self::PROJECT_END);
        Self::PROJECT.replace(Self::PROJECT_END, &reference)
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpWebProject;

    #[test]
    fn fsharp_records_join_the_model_context_only_with_fsharp_services() {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("root: {error}"));
        let models = root.path().join("client/models");
        std::fs::create_dir_all(&models).unwrap_or_else(|error| panic!("models: {error}"));
        std::fs::write(
            models.join("SignalSent.fs"),
            "type SignalSent = { Size: int }\n",
        )
        .unwrap_or_else(|error| panic!("model: {error}"));
        let context = "[A]\ninternal partial class TacModelContext : Base {}\n";
        let unchanged = FsharpWebProject::with_model_context(context, root.path(), "")
            .unwrap_or_else(|error| panic!("context: {error}"));
        assert_eq!(unchanged, context);
        let joined = FsharpWebProject::with_model_context(
            context,
            root.path(),
            "namespace TachyonService0\n",
        )
        .unwrap_or_else(|error| panic!("context: {error}"));
        assert_eq!(
            joined,
            "[A]\n[System.Text.Json.Serialization.JsonSerializable(typeof(Models.SignalSent))]\ninternal partial class TacModelContext : Base {}\n"
        );
    }

    #[test]
    fn only_a_page_with_fsharp_services_references_the_fsharp_project() {
        let project = "<Project>\n</Project>\n";
        assert_eq!(FsharpWebProject::referenced(project, ""), project);
        let referenced = FsharpWebProject::referenced(project, "namespace TachyonService0\n");
        assert!(referenced.contains("FsharpServices/FsharpServices.fsproj"));
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("root: {error}"));
        assert!(
            FsharpWebProject::files(root.path(), "", "")
                .unwrap_or_else(|error| panic!("files: {error}"))
                .is_empty()
        );
        let files = FsharpWebProject::files(root.path(), "namespace TachyonService0\n", "")
            .unwrap_or_else(|error| panic!("files: {error}"));
        assert!(
            files[1]
                .1
                .starts_with("namespace rec Models\nnamespace TachyonService0")
        );
        let files = FsharpWebProject::files(
            root.path(),
            "namespace TachyonService0\n",
            "namespace TachyonService0 {\n}\n",
        )
        .unwrap_or_else(|error| panic!("files: {error}"));
        assert!(
            files[0]
                .1
                .contains("../FsharpBehaviors/FsharpBehaviors.csproj")
        );
        assert_eq!(files[3].0, "FsharpBehaviors/FsharpBehaviors.csproj");
        assert!(
            files[4]
                .1
                .contains("InternalsVisibleTo(\"FsharpServices\")")
        );
    }

    #[test]
    fn fsharp_services_see_the_browser_apis_and_their_events() {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("root: {error}"));
        let files = FsharpWebProject::files(root.path(), "namespace TachyonService0\n", "")
            .unwrap_or_else(|error| panic!("files: {error}"));
        let named = |wanted: &str| {
            files
                .iter()
                .find(|(name, _)| name == wanted)
                .map(|(_, contents)| contents.clone())
                .unwrap_or_else(|| panic!("no {wanted} in {files:?}"))
        };
        let project = named("FsharpServices/FsharpServices.fsproj");
        assert!(
            project.contains(
                "<Compile Include=\"WebEvents.fs\" /><Compile Include=\"Services.fs\" />"
            )
        );
        assert!(project.contains("../FsharpBehaviors/FsharpBehaviors.csproj"));
        assert!(
            named("FsharpServices/WebEvents.fs")
                .contains("type ConnectivityChanged = { Online: bool }")
        );
        assert!(named("FsharpBehaviors/WebApis.cs").contains("public sealed class Clock"));
    }
}
