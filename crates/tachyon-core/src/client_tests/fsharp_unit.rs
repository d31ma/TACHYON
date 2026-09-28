use super::{dotnet_framework, execute, fail, stage_failure};
use crate::Failure;
use crate::fsharp_source::{FsharpProject, FsharpSource, FsharpTestUnit, FsharpView};
use crate::model_sources::ModelSources;
use std::path::Path;
use std::process::Command;

pub(super) struct FsharpUnit;

impl FsharpUnit {
    const UNIT: &'static str = "unit";
    const MAIN: &'static str = "\nstatic class TachyonTestMain\n{\n    static void Main() => Console.WriteLine(System.Text.Json.JsonSerializer.Serialize(Handler.POST()));\n}\n";
    const NO_UNIT: &'static str =
        "A test unit declares Test on one class; this source declares none.";

    pub(super) fn run(
        project: &Path,
        relative: &Path,
        source: &str,
        portable: &str,
    ) -> Result<String, Failure> {
        let assembled = crate::annotation_modules::assemble(project, relative, source, "tac")?
            .unwrap_or_else(|| source.to_owned());
        let view = FsharpView::new(relative, &assembled);
        let lowered = FsharpTestUnit::lower(relative, &assembled, &view, "tac")?
            .ok_or_else(|| fail(portable, Self::NO_UNIT))?;
        let models = ModelSources::read(project, "client", FsharpSource::EXTENSION)?;
        let staging = tempfile::Builder::new()
            .prefix("tachyon-fsharp-unit-")
            .tempdir()
            .map_err(|error| stage_failure(portable, &error))?;
        Self::write(staging.path(), portable, &lowered.source, &models)?;
        Self::build_and_run(staging.path(), portable)
    }

    fn write(
        directory: &Path,
        portable: &str,
        program: &str,
        models: &ModelSources,
    ) -> Result<(), Failure> {
        let (fsharp, csharp) = FsharpSource::split(program);
        let framework = dotnet_framework(portable)?;
        let library = directory.join(FsharpProject::LIBRARY_FOLDER);
        let reference =
            FsharpProject::reference(&format!("{}/service.fsproj", FsharpProject::LIBRARY_FOLDER));
        let files = [
            (
                library.join(FsharpProject::MODELS_FILE),
                FsharpSource::models_file(&models.separate_files()),
            ),
            (
                library.join(FsharpProject::PROGRAM_FILE),
                FsharpProject::program_file(portable, fsharp),
            ),
            (
                library.join("service.fsproj"),
                FsharpProject::library(&framework, ""),
            ),
            (directory.join("unit.cs"), format!("{csharp}{}", Self::MAIN)),
            (
                directory.join("unit.csproj"),
                FsharpProject::executable(&framework, Self::UNIT, &reference),
            ),
        ];
        std::fs::create_dir(&library).map_err(|error| stage_failure(portable, &error))?;
        for (path, contents) in files {
            std::fs::write(path, contents).map_err(|error| stage_failure(portable, &error))?;
        }
        Ok(())
    }

    fn build_and_run(directory: &Path, portable: &str) -> Result<String, Failure> {
        execute(
            Command::new("dotnet")
                .current_dir(directory)
                .args(["build", "-c", "Release", "-o", "out", "--nologo", "-v", "q"])
                .arg("--disable-build-servers"),
            portable,
        )?;
        execute(
            Command::new("dotnet").arg(directory.join("out").join("unit.dll")),
            portable,
        )
    }
}
