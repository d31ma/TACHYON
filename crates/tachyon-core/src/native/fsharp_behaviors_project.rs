pub(super) struct FsharpBehaviorsProject;

impl FsharpBehaviorsProject {
    pub(super) const REFERENCE: &'static str = "  <ItemGroup><ProjectReference Include=\"../FsharpBehaviors/FsharpBehaviors.csproj\" /></ItemGroup>\n";
    const FOLDER: &'static str = "FsharpBehaviors";
    const ACCESS: &'static str = "[assembly: System.Runtime.CompilerServices.InternalsVisibleTo(\"FsharpServices\")]\n[assembly: System.Runtime.CompilerServices.InternalsVisibleTo(\"TachyonServices\")]\n";
    const PROJECT: &'static str = "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net9.0</TargetFramework>\n    <OutputType>Library</OutputType>\n    <AssemblyName>FsharpBehaviors</AssemblyName>\n    <Nullable>disable</Nullable>\n    <ImplicitUsings>enable</ImplicitUsings>\n    <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n  </PropertyGroup>\n  <ItemGroup><Compile Include=\"Behaviors.cs\" /></ItemGroup>\n</Project>\n";

    pub(super) fn files(runtime: &str) -> Vec<(String, String)> {
        let files = (!runtime.is_empty()).then(|| Self::written(runtime));
        files.unwrap_or_default()
    }

    fn written(runtime: &str) -> Vec<(String, String)> {
        vec![
            (
                format!("{}/FsharpBehaviors.csproj", Self::FOLDER),
                String::from(Self::PROJECT),
            ),
            (
                format!("{}/Behaviors.cs", Self::FOLDER),
                format!("{}{runtime}", Self::ACCESS),
            ),
        ]
    }
}
