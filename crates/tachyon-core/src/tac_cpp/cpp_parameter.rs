use super::CppAttribute;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CppParameter {
    pub(crate) name: String,
    pub(crate) type_text: String,
    pub(crate) markers: Vec<CppAttribute>,
}
