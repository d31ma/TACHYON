#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum RuntimeRequirementKey {
    JavaScript,
    Python,
    Ruby,
    Java,
    GroovyRuntime,
    Php,
    KotlinCompiler,
    Dotnet,
    RustCompiler,
    DartCompiler,
    GoCompiler,
    FirecrackerDriver,
    GatewayProgram,
}
