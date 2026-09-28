use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::groovy_source::GroovySource;
use std::path::Path;

pub(crate) struct TypedMembers;

impl TypedMembers {
    pub(crate) const CODE: u16 = 1334;
    const UNTYPED: &'static str = "def";
    const CLASS_BODY_DEPTH: usize = 1;
    const DECLARATION_ENDS: [char; 3] = ['(', '=', '{'];
    const HELP: &'static str = "Write the type in place of def, for example `String name` or `List<Order> findOrders()`. Tachyon reads member types to check routes, models and injection (ADR 0072).";

    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        let code = Self::groovy_code(path, source)?;
        let found = Self::first_untyped_member(&code);
        Self::refuse(path, found)
    }

    fn groovy_code(path: &Path, source: &str) -> Result<String, Failure> {
        if !GroovySource::is_groovy(path) {
            return Ok(String::new());
        }
        super::masked_source(path, source)
    }

    fn first_untyped_member(code: &str) -> Option<usize> {
        let mut found = Vec::new();
        let mut depth = 0_usize;
        let mut offset = 0_usize;
        for line in code.split_inclusive('\n') {
            let in_class_body = depth == Self::CLASS_BODY_DEPTH;
            found.extend(Self::untyped_member_at(line, in_class_body, offset));
            depth = super::brace_depth_after(line, depth);
            offset += line.len();
        }
        found.first().copied()
    }

    fn untyped_member_at(line: &str, in_class_body: bool, offset: usize) -> Option<usize> {
        let member = in_class_body.then_some(line)?;
        let indent = member.len() - member.trim_start().len();
        Self::declares_def(member).then_some(offset + indent)
    }

    fn declares_def(line: &str) -> bool {
        let declaration = line
            .split(Self::DECLARATION_ENDS)
            .next()
            .unwrap_or_default();
        declaration
            .split_whitespace()
            .any(|word| word == Self::UNTYPED)
    }

    fn refuse(path: &Path, found: Option<usize>) -> Result<(), Failure> {
        let Some(at) = found else {
            return Ok(());
        };
        Err(Self::failure(path, at))
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
            "@Needs def ledgerStore",
        ] {
            let source = format!("@Service\nclass OrderService {{\n    {member}\n}}\n");
            let failure = checked(&source).err().unwrap_or_default();
            assert!(failure.contains("TY1334"), "{member}: {failure}");
        }
    }

    #[test]
    fn typed_members_and_local_def_variables_are_allowed() {
        let source = "@Service\nclass OrderService {\n    List<String> orders = []\n    int size() { def count = orders.size(); return count }\n    String name() {\n        def local = 'def'\n        return local\n    }\n}\n";
        assert!(checked(source).is_ok());
        let java = Path::new("server/services/OrderService.java");
        assert!(TypedMembers::check(java, "class A {\n    def x;\n}\n").is_ok());
    }
}
