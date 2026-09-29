use super::fsharp_behaviors_project::FsharpBehaviorsProject;
use crate::Failure;
use crate::fsharp_source::FsharpSource;
use crate::model_sources::ModelSources;
use std::path::Path;

pub(super) struct FsharpWebProject;

impl FsharpWebProject {
    const FOLDER: &'static str = "FsharpServices";
    const PROJECT_END: &'static str = "</Project>";
    const REFERENCE: &'static str = "  <ItemGroup><ProjectReference Include=\"FsharpServices/FsharpServices.fsproj\" /></ItemGroup>\n";
    const PROJECT: &'static str = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net9.0</TargetFramework>\n    <OutputType>Library</OutputType>\n    <AssemblyName>FsharpServices</AssemblyName>\n    <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n    <SatelliteResourceLanguages>en</SatelliteResourceLanguages>\n  </PropertyGroup>\n  <ItemGroup><Compile Include=\"Services.fs\" /></ItemGroup>\n</Project>\n";

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
                Self::services_project(runtime),
            ),
            (format!("{}/Services.fs", Self::FOLDER), services),
        ];
        files.extend(FsharpBehaviorsProject::files(runtime));
        Ok(Self::wanted(files, fsharp))
    }

    fn services_project(runtime: &str) -> String {
        if runtime.is_empty() {
            return String::from(Self::PROJECT);
        }
        let reference = format!("{}{}", FsharpBehaviorsProject::REFERENCE, Self::PROJECT_END);
        Self::PROJECT.replace(Self::PROJECT_END, &reference)
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpWebProject;

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
        assert_eq!(files[2].0, "FsharpBehaviors/FsharpBehaviors.csproj");
        assert!(
            files[3]
                .1
                .contains("InternalsVisibleTo(\"FsharpServices\")")
        );
    }
}
