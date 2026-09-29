use super::java_code::JavaCode;

pub(super) struct JavaEnclosure;

impl JavaEnclosure {
    const MODEL_IMPORT: &'static str = "import models.";
    const MEMBER_MARK: &'static str = "static ";
    const CLOSING: &'static str = "}\n";
    const BLANK: u8 = b' ';
    const STATEMENT_END: u8 = b';';

    pub(super) fn holder(index: usize) -> String {
        format!("TacService{index}")
    }

    pub(super) fn wrap(source: &str, holder: &str) -> String {
        let without_imports = Self::without_model_imports(source);
        let code = JavaCode::new(&without_imports);
        let types = code.top_level_types();
        let opening = types
            .first()
            .map_or(0, |first| code.statement_end_before(first.keyword_at));
        let mut inserts = vec![(opening, format!(" final class {holder} {{ "))];
        inserts.extend(
            types
                .iter()
                .map(|declaration| (declaration.keyword_at, String::from(Self::MEMBER_MARK))),
        );
        let mut wrapped = Self::inserted(&without_imports, &inserts);
        wrapped.push_str(Self::CLOSING);
        wrapped
    }

    fn inserted(source: &str, inserts: &[(usize, String)]) -> String {
        let mut wrapped = String::with_capacity(source.len() + 64);
        let mut from = 0;
        for (at, text) in inserts {
            wrapped.push_str(&source[from..*at]);
            wrapped.push_str(text);
            from = *at;
        }
        wrapped.push_str(&source[from..]);
        wrapped
    }

    fn without_model_imports(source: &str) -> String {
        let code = JavaCode::new(source);
        let text = String::from_utf8_lossy(code.bytes()).into_owned();
        let mut bytes = source.as_bytes().to_vec();
        for (at, _) in text.match_indices(Self::MODEL_IMPORT) {
            let end = code.next(at, Self::STATEMENT_END).unwrap_or(at);
            bytes[at..=end].fill(Self::BLANK);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::JavaEnclosure;

    #[test]
    fn a_service_file_becomes_one_holder_class_on_the_same_lines() {
        let source = "import java.util.List;\nimport models.Guide;\n\nfinal class NotesService {\n    Guide guide;\n}\nfinal class Behaviors {\n    interface Call {}\n    enum Kind { ONE }\n}\nrecord Pair(int left) {}\n";
        let wrapped = JavaEnclosure::wrap(source, "TacService3");
        assert_eq!(
            wrapped,
            "import java.util.List; final class TacService3 { \n                    \n\nfinal static class NotesService {\n    Guide guide;\n}\nfinal static class Behaviors {\n    interface Call {}\n    enum Kind { ONE }\n}\nstatic record Pair(int left) {}\n}\n"
        );
        assert_eq!(
            wrapped.lines().count(),
            source.lines().count() + 1,
            "{wrapped}"
        );
    }
}
