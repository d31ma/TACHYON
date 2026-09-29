use super::fsharp_program::FsharpProgram;
use super::{
    CacheDirectory, DotnetCacheEntry, EntryClass, Failure, PreparedCommand, compiler_identity,
    compiler_workspace, csharp_digest, installed_dotnet_framework, invalid_source, lock_for_build,
    prepare_dotnet_runtime, publish_dotnet_cache, remove_incomplete_csharp_entry, run_dotnet_build,
    valid_dotnet_build, valid_dotnet_cache, write_compiler_file,
};
use crate::fsharp_source::{FsharpProject, FsharpSource};
use std::fs;
use std::path::{Path, PathBuf};

type BuildStep =
    fn(&CacheDirectory, &DotnetCacheEntry<'_>, &FsharpProgram<'_>) -> Result<(), Failure>;

pub(super) struct FsharpHandler;

impl FsharpHandler {
    const FILES: [&'static str; 6] = [
        "handler.dll",
        "handler.deps.json",
        "handler.runtimeconfig.json",
        "yon.dll",
        "service.dll",
        "FSharp.Core.dll",
    ];
    const PRELUDE: &'static str = include_str!("../preludes/yon.cs");
    const ENTRY_PLACEHOLDER: &'static str = "__YON_HANDLER__";
    const PRELUDE_ACCESS: &'static str = "[assembly: System.Runtime.CompilerServices.InternalsVisibleTo(\"service\")]\n[assembly: System.Runtime.CompilerServices.InternalsVisibleTo(\"handler\")]\n";
    const PRELUDE_FOLDER: &'static str = "yon";
    const RUNTIME_FILE: &'static str = "behaviors.cs";
    const BUILD_PREFIX: &'static str = "tachyon-fsharp-build-";

    pub(super) fn compile(
        bytes: &[u8],
        portable: &str,
        cache: &CacheDirectory,
        models: &[(PathBuf, String)],
    ) -> Result<PreparedCommand, Failure> {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let (fsharp, generated) = FsharpSource::split(&text);
        let (csharp, runtime) = FsharpSource::runtime_split(generated);
        let framework = installed_dotnet_framework(portable)?;
        let program = FsharpProgram {
            fsharp,
            csharp,
            runtime,
            models,
            entry: EntryClass::of(portable),
            portable,
            framework: &framework,
        };
        let digest = csharp_digest(&Self::cache_key(&program), &framework);
        let metadata = format!("digest={digest}\nframework={framework}\n");
        let entry = DotnetCacheEntry {
            digest: &digest,
            metadata: &metadata,
            files: &Self::FILES,
        };
        Self::unless_cached(cache, &entry, &program, Self::locked_build)?;
        prepare_dotnet_runtime(cache, &entry, portable)
    }

    fn cache_key(program: &FsharpProgram<'_>) -> Vec<u8> {
        let mut keyed = FsharpSource::models_file(program.models);
        keyed.push_str(program.fsharp);
        keyed.push_str(&Self::entry_file(program));
        keyed.push_str(program.runtime);
        keyed.push_str(Self::PRELUDE);
        keyed.push_str(&compiler_identity("dotnet"));
        keyed.push_str(std::env::consts::ARCH);
        keyed.push_str(std::env::consts::OS);
        keyed.into_bytes()
    }

    fn unless_cached(
        cache: &CacheDirectory,
        entry: &DotnetCacheEntry<'_>,
        program: &FsharpProgram<'_>,
        work: BuildStep,
    ) -> Result<(), Failure> {
        if valid_dotnet_cache(cache, entry.digest, entry.metadata, entry.files) {
            return Ok(());
        }
        work(cache, entry, program)
    }

    fn locked_build(
        cache: &CacheDirectory,
        entry: &DotnetCacheEntry<'_>,
        program: &FsharpProgram<'_>,
    ) -> Result<(), Failure> {
        let portable = program.portable;
        let lock = lock_for_build(
            cache,
            entry.digest,
            portable,
            "Cannot coordinate F# compilation",
        )?;
        let built = Self::unless_cached(cache, entry, program, Self::build);
        drop(lock);
        built
    }

    fn build(
        cache: &CacheDirectory,
        entry: &DotnetCacheEntry<'_>,
        program: &FsharpProgram<'_>,
    ) -> Result<(), Failure> {
        let portable = program.portable;
        remove_incomplete_csharp_entry(cache, entry.digest, portable)?;
        let build = compiler_workspace(
            Self::BUILD_PREFIX,
            "Cannot create F# compiler workspace",
            portable,
        )?;
        Self::write_projects(build.path(), program)?;
        run_dotnet_build(build.path(), portable)?;
        let output = build.path().join("out");
        Self::check_output(&output, portable)?;
        publish_dotnet_cache(cache, entry, &output, portable)
    }

    fn check_output(output: &Path, portable: &str) -> Result<(), Failure> {
        if valid_dotnet_build(output, &Self::FILES) {
            return Ok(());
        }
        Err(invalid_source(
            portable,
            "Compiler completed without producing the complete F# runtime artifact set.",
        ))
    }

    fn write_projects(build: &Path, program: &FsharpProgram<'_>) -> Result<(), Failure> {
        let portable = program.portable;
        let framework = program.framework;
        let prelude = build.join(Self::PRELUDE_FOLDER);
        let service = build.join(FsharpProject::LIBRARY_FOLDER);
        Self::create_folder(&prelude, portable)?;
        Self::create_folder(&service, portable)?;
        write_compiler_file(
            &prelude.join("yon.cs"),
            Self::prelude_library().as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &prelude.join(Self::RUNTIME_FILE),
            program.runtime.as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &prelude.join("access.cs"),
            Self::PRELUDE_ACCESS.as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &prelude.join("yon.csproj"),
            Self::prelude_project(framework).as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &service.join(FsharpProject::MODELS_FILE),
            FsharpSource::models_file(program.models).as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &service.join(FsharpProject::PROGRAM_FILE),
            FsharpProject::program_file(program.portable, program.fsharp).as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &service.join("service.fsproj"),
            FsharpProject::library(framework, &Self::prelude_reference()).as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &build.join("handler.cs"),
            Self::entry_file(program).as_bytes(),
            portable,
        )?;
        write_compiler_file(
            &build.join("handler.csproj"),
            FsharpProject::executable(framework, "handler", &Self::entry_references()).as_bytes(),
            portable,
        )
    }

    fn create_folder(folder: &Path, portable: &str) -> Result<(), Failure> {
        fs::create_dir(folder).map_err(|error| {
            invalid_source(
                portable,
                &format!("Cannot create F# compiler folder: {error}"),
            )
        })
    }

    fn prelude_library() -> String {
        let mut library = String::new();
        for line in Self::PRELUDE.lines() {
            Self::keep_library_line(&mut library, line);
        }
        library
    }

    fn keep_library_line(library: &mut String, line: &str) {
        if line.contains(Self::ENTRY_PLACEHOLDER) {
            return;
        }
        library.push_str(line);
        library.push('\n');
    }

    fn prelude_reference() -> String {
        FsharpProject::reference(&format!("../{}/yon.csproj", Self::PRELUDE_FOLDER))
    }

    fn entry_references() -> String {
        let prelude = FsharpProject::reference(&format!("{}/yon.csproj", Self::PRELUDE_FOLDER));
        let library =
            FsharpProject::reference(&format!("{}/service.fsproj", FsharpProject::LIBRARY_FOLDER));
        format!("{prelude}{library}")
    }

    fn entry_file(program: &FsharpProgram<'_>) -> String {
        format!(
            "{}\nstatic class __YonProgram\n{{\n    static System.Threading.Tasks.Task Main() => Yon.Run(typeof({}));\n}}\n",
            program.csharp,
            program.entry.name()
        )
    }

    fn prelude_project(framework: &str) -> String {
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
             <OutputType>Library</OutputType>\n    \
             <TargetFramework>{framework}</TargetFramework>\n    \
             <AssemblyName>yon</AssemblyName>\n    \
             <Nullable>disable</Nullable>\n    \
             <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n    \
             <ImplicitUsings>enable</ImplicitUsings>\n  \
             </PropertyGroup>\n  <ItemGroup><Compile Include=\"yon.cs\" />\
             <Compile Include=\"access.cs\" /><Compile Include=\"behaviors.cs\" /></ItemGroup>\n\
             </Project>\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{EntryClass, FsharpHandler, FsharpProgram};
    use crate::fsharp_source::{FsharpProject, FsharpSource};
    use std::path::PathBuf;

    fn program<'program>(
        fsharp: &'program str,
        csharp: &'program str,
        models: &'program [(PathBuf, String)],
    ) -> FsharpProgram<'program> {
        FsharpProgram {
            fsharp,
            csharp,
            runtime: "",
            models,
            entry: EntryClass::Handler,
            portable: "server/controllers/OrdersController.fs",
            framework: "net9.0",
        }
    }

    #[test]
    fn the_prelude_library_has_no_entry_point() {
        let library = FsharpHandler::prelude_library();
        assert!(!library.contains("__YON_HANDLER__"));
        assert!(library.contains("internal static async Task Run(Type entry)"));
    }

    #[test]
    fn the_program_keeps_the_author_line_numbers_and_the_entry_starts_the_handler() {
        let models = [(
            PathBuf::from("server/models/Order.fs"),
            String::from("type Order = { Id: int }"),
        )];
        let program = program(
            "type Handler() = class end",
            "sealed class Glue {}",
            &models,
        );
        let fsharp = FsharpProject::program_file(program.portable, program.fsharp);
        assert!(
            fsharp.starts_with(
                "namespace rec global\n# 1 \"server/controllers/OrdersController.fs\"\n"
            )
        );
        let models = FsharpSource::models_file(program.models);
        assert!(models.contains("# 1 \"server/models/Order.fs\"\ntype Order = { Id: int }"));
        let entry = FsharpHandler::entry_file(&program);
        assert!(entry.starts_with("sealed class Glue {}"));
        assert!(entry.contains("Yon.Run(typeof(Handler))"));
    }
}
