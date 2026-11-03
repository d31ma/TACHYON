#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KotlinMember {
    pub(crate) name: String,
    pub(crate) value_type: String,
    pub(crate) parameters: Vec<String>,
    pub(crate) method: bool,
    pub(crate) mutable: bool,
}
