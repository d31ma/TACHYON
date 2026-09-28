use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::path::Path;

pub(crate) struct TypedMembers;

impl TypedMembers {
    pub(crate) const CODE: u16 = 1334;
    const LANGUAGE: &'static str = "groovy";
    const UNTYPED: &'static str = "def";
    const CLASS_BODY_DEPTH: usize = 1;
    const MODIFIERS: [&'static str; 8] = [
        "public",
        "private",
        "protected",
        "static",
        "final",
        "abstract",
        "synchronized",
        "transient",
    ];
    const HELP: &'static str = "Write the type in place of def, for example `String name` or `List<Order> findOrders()`. Tachyon reads member types to check routes, models and injection (ADR 0072).";

    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        let extension = path.extension().and_then(|value| value.to_str());
        if extension != Some(Self::LANGUAGE) {
            return Ok(());
        }
        let code = super::masked_source(path, source)?;
        let mut depth = 0_usize;
        let mut offset = 0_usize;
        for line in code.split_inclusive('\n') {
            if depth == Self::CLASS_BODY_DEPTH && Self::starts_with_def(line) {
                let at = offset + line.len() - line.trim_start().len();
                return Err(Self::failure(path, at));
            }
            depth = super::brace_depth_after(line, depth);
            offset += line.len();
        }
        Ok(())
    }

    fn starts_with_def(line: &str) -> bool {
        for word in line.split_whitespace() {
            if Self::MODIFIERS.contains(&word) || word.starts_with('@') {
                continue;
            }
            return word == Self::UNTYPED;
        }
        false
    }

    fn failure(path: &Path, at: usize) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            Self::CODE,
            format!("A Groovy class member in '{portable}' uses def; give it a declared type"),
            Some(String::from(Self::HELP)),
            source_span(&portable, at, Self::UNTYPED.len()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::TypedMembers;
    use std::path::Path;

    fn checked(source: &str) -> Result<(), String> {
        let path = Path::new("server/services/OrderService.groovy");
        TypedMembers::check(path, source).map_err(|failure| failure.to_string())
    }

    #[test]
    fn a_member_declared_with_def_is_refused() {
        for member in [
            "def orders = []",
            "private def orders",
            "static def find() { null }",
        ] {
            let source = format!("@Service\nclass OrderService {{\n    {member}\n}}\n");
            let failure = checked(&source).err().unwrap_or_default();
            assert!(failure.contains("TY1334"), "{member}: {failure}");
        }
    }

    #[test]
    fn typed_members_and_local_def_variables_are_allowed() {
        let source = "@Service\nclass OrderService {\n    List<String> orders = []\n    String name() {\n        def local = 'def'\n        return local\n    }\n}\n";
        assert!(checked(source).is_ok());
        let java = Path::new("server/services/OrderService.java");
        assert!(TypedMembers::check(java, "class A {\n    def x;\n}\n").is_ok());
    }
}
