pub(crate) struct FsharpProject;

impl FsharpProject {
    pub(crate) const MODELS_FILE: &'static str = "Models.fs";
    pub(crate) const PROGRAM_FILE: &'static str = "Program.fs";
    pub(crate) const LIBRARY_FOLDER: &'static str = "service";
    pub(crate) const BEHAVIORS_FOLDER: &'static str = "behaviors";
    const PROGRAM_NAMESPACE: &'static str = "namespace rec global\n";
    const BEHAVIORS_ACCESS: &'static str = "[assembly: System.Runtime.CompilerServices.InternalsVisibleTo(\"service\")]\n[assembly: System.Runtime.CompilerServices.InternalsVisibleTo(\"unit\")]\n";

    pub(crate) fn program_file(portable: &str, fsharp: &str) -> String {
        format!(
            "{}{}{fsharp}\n",
            Self::PROGRAM_NAMESPACE,
            super::FsharpLines::directive(1, portable)
        )
    }

    pub(crate) fn reference(project: &str) -> String {
        format!("  <ItemGroup><ProjectReference Include=\"{project}\" /></ItemGroup>\n")
    }

    pub(crate) fn library(framework: &str, references: &str) -> String {
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
             <OutputType>Library</OutputType>\n    \
             <TargetFramework>{framework}</TargetFramework>\n    \
             <AssemblyName>{}</AssemblyName>\n    \
             <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n    \
             <SatelliteResourceLanguages>en</SatelliteResourceLanguages>\n  \
             </PropertyGroup>\n  <ItemGroup><Compile Include=\"{}\" />\
             <Compile Include=\"{}\" /></ItemGroup>\n{references}</Project>\n",
            Self::LIBRARY_FOLDER,
            Self::MODELS_FILE,
            Self::PROGRAM_FILE,
        )
    }

    pub(crate) fn behaviors_file(runtime: &str) -> String {
        format!("{}{runtime}", Self::BEHAVIORS_ACCESS)
    }

    pub(crate) fn behaviors_library(framework: &str) -> String {
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
             <OutputType>Library</OutputType>\n    \
             <TargetFramework>{framework}</TargetFramework>\n    \
             <AssemblyName>{}</AssemblyName>\n    \
             <Nullable>disable</Nullable>\n    \
             <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n    \
             <ImplicitUsings>enable</ImplicitUsings>\n    \
             <SatelliteResourceLanguages>en</SatelliteResourceLanguages>\n  \
             </PropertyGroup>\n  <ItemGroup><Compile Include=\"{}.cs\" /></ItemGroup>\n</Project>\n",
            Self::BEHAVIORS_FOLDER,
            Self::BEHAVIORS_FOLDER,
        )
    }

    pub(crate) fn executable(framework: &str, name: &str, references: &str) -> String {
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
             <OutputType>Exe</OutputType>\n    \
             <TargetFramework>{framework}</TargetFramework>\n    \
             <AssemblyName>{name}</AssemblyName>\n    \
             <Nullable>disable</Nullable>\n    \
             <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n    \
             <ImplicitUsings>enable</ImplicitUsings>\n    \
             <SatelliteResourceLanguages>en</SatelliteResourceLanguages>\n  \
             </PropertyGroup>\n  <ItemGroup><Compile Include=\"{name}.cs\" /></ItemGroup>\n\
             {references}</Project>\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpProject;

    #[test]
    fn the_library_compiles_models_before_the_program_and_keeps_line_numbers() {
        let library =
            FsharpProject::library("net9.0", &FsharpProject::reference("../yon/yon.csproj"));
        let models = library
            .find("Models.fs")
            .unwrap_or_else(|| panic!("models"));
        assert!(
            models
                < library
                    .find("Program.fs")
                    .unwrap_or_else(|| panic!("program"))
        );
        assert!(library.contains("../yon/yon.csproj"));
        let program = FsharpProject::program_file("server/services/A.fs", "type A() = class end");
        assert!(program.starts_with("namespace rec global\n# 1 \"server/services/A.fs\"\n"));
        assert!(
            FsharpProject::executable("net9.0", "unit", "")
                .contains("<Compile Include=\"unit.cs\" />")
        );
    }
}
