use super::java_model_kind::JavaModelKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct JavaModel {
    pub(super) name: String,
    pub(super) kind: JavaModelKind,
    pub(super) fields: Vec<(String, String)>,
}
