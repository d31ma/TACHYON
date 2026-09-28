use super::{
    BraceBody, DartBody, KotlinBody, MethodHeader, PublicMethod, PythonBody, RoleClass, RubyBody,
    SourceEdit, SwiftBody,
};

pub(crate) struct MethodBody;

impl MethodBody {
    pub(crate) fn edits(role: &RoleClass, source: &str, method: &PublicMethod) -> Vec<SourceEdit> {
        let header = MethodHeader::new(role, source, method);
        Self::placed(&role.language, &header)
    }

    fn placed(language: &str, header: &MethodHeader<'_>) -> Vec<SourceEdit> {
        match language {
            "py" => PythonBody::edits(header),
            "rb" => RubyBody::edits(header),
            "kt" => KotlinBody::edits(header),
            "dart" => DartBody::edits(header),
            "swift" => SwiftBody::edits(header),
            _ => BraceBody::edits(header),
        }
    }
}
