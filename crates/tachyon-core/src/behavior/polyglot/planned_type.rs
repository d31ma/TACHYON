#[derive(Clone, Debug)]
pub(crate) struct PlannedType {
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) declared: bool,
    pub(crate) takes_context: bool,
}
