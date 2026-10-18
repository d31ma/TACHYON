#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum RuntimeRequirementKey {
    JavaScript,
    NodeProxySwitch,
    Python,
    Ruby,
    Lua,
    Java,
    GroovyRuntime,
    Php,
    KotlinCompiler,
    Dotnet,
    RustCompiler,
    DartCompiler,
    GoCompiler,
    CppCompiler,
    FirecrackerDriver,
    GatewayProgram,
}
