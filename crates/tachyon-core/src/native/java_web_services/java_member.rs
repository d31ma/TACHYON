use super::java_member_kind::JavaMemberKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct JavaMember {
    pub(super) name: String,
    pub(super) value_type: String,
    pub(super) parameters: Vec<String>,
    pub(super) kind: JavaMemberKind,
}
