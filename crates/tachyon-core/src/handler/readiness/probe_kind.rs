#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProbeKind {
    JavaScript { typescript: bool },
    NodeProxySwitch,
    Python,
    RubyVersion,
    LuaVersion,
    Java { source_launcher: bool },
    Version(&'static [&'static str]),
    GoVersion,
    DotnetBuild,
    File,
    OnPath,
}
