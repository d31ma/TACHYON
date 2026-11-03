#[derive(Clone, Copy, Debug)]
pub(crate) struct ToolEntry {
    pub(crate) label: &'static str,
    pub(crate) commands: &'static [&'static str],
}
