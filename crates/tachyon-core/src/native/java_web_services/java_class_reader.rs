use super::java_declaration::JavaDeclaration;
use super::java_member::JavaMember;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::ops::Range;

pub(super) struct JavaClassReader<'a> {
    path: &'a str,
    code: Vec<u8>,
    braces: Vec<i32>,
    parens: Vec<i32>,
}

impl<'a> JavaClassReader<'a> {
    const LANGUAGE: &'static str = "java";
    const SHAPE_CODE: u16 = 1405;
    const OWNER_CODE: u16 = 1406;
    const OPEN: u8 = b'{';
    const CLOSE: u8 = b'}';
    const STATEMENT_END: u8 = b';';
    const OPEN_ARGUMENTS: u8 = b'(';
    const CLOSE_ARGUMENTS: u8 = b')';
    const MARKERS: [&'static str; 4] = ["@OnStart", "@OnStop", "@LocalState", "@SessionState"];
    const BLANK: u8 = b' ';

    pub(super) fn new(path: &'a str, source: &str) -> Self {
        let code = Self::visible(source);
        let braces = Self::depths(&code, Self::OPEN, Self::CLOSE);
        let parens = Self::depths(&code, Self::OPEN_ARGUMENTS, Self::CLOSE_ARGUMENTS);
        Self {
            path,
            code,
            braces,
            parens,
        }
    }

    pub(super) fn members(&self, class: &str) -> Result<Vec<JavaMember>, Failure> {
        let declared = self
            .class_declaration(class)
            .ok_or_else(|| self.missing_class(class))?;
        self.check_class_header(declared)?;
        let body = self
            .class_body(declared)
            .ok_or_else(|| self.missing_class(class))?;
        let headers = self.headers(&body);
        let members = self.declared(&headers)?;
        self.require_members(members, class)
    }

    pub(super) fn without_markers(&self, source: &str) -> String {
        let spans = self.marker_spans();
        Self::blanked(source, &spans)
    }

    fn marker_spans(&self) -> Vec<Range<usize>> {
        Self::MARKERS
            .iter()
            .flat_map(|marker| self.spans_of(marker))
            .collect()
    }

    fn spans_of(&self, marker: &str) -> Vec<Range<usize>> {
        let text = String::from_utf8_lossy(&self.code);
        text.match_indices(marker)
            .map(|(at, _)| at..at + marker.len())
            .filter(|span| self.ends_word(span.end))
            .map(|span| self.with_arguments(span))
            .collect()
    }

    fn with_arguments(&self, span: Range<usize>) -> Range<usize> {
        if self.code.get(span.end) != Some(&Self::OPEN_ARGUMENTS) {
            return span;
        }
        let close = Self::closing(&self.parens, span.end).unwrap_or(span.end);
        span.start..close + 1
    }

    fn blanked(source: &str, spans: &[Range<usize>]) -> String {
        let mut bytes = source.as_bytes().to_vec();
        for span in spans {
            bytes[span.clone()].fill(Self::BLANK);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn visible(source: &str) -> Vec<u8> {
        let mask = crate::lexical::code_mask(Self::LANGUAGE, source);
        source
            .bytes()
            .zip(mask.code)
            .map(|(byte, visible)| if visible { byte } else { Self::BLANK })
            .collect()
    }

    fn depths(code: &[u8], open: u8, close: u8) -> Vec<i32> {
        let mut depth = 0;
        let mut depths = Vec::with_capacity(code.len() + 1);
        for byte in code {
            depths.push(depth);
            depth += i32::from(*byte == open) - i32::from(*byte == close);
        }
        depths.push(depth);
        depths
    }

    fn check_class_header(&self, declared: usize) -> Result<(), Failure> {
        let header = self.header_start(declared)..declared;
        let text = String::from_utf8_lossy(&self.code[header.clone()]);
        JavaDeclaration::parse(&text)
            .refuse_class_annotations()
            .map_err(|message| self.failure(Self::SHAPE_CODE, message, &header))
    }

    fn header_start(&self, declared: usize) -> usize {
        (0..declared)
            .rev()
            .find(|at| matches!(self.code[*at], Self::STATEMENT_END | Self::CLOSE))
            .map_or(0, |at| at + 1)
    }

    fn class_body(&self, declared: usize) -> Option<Range<usize>> {
        let open = self.next_open_brace(declared)?;
        let close = Self::closing(&self.braces, open)?;
        Some(open + 1..close)
    }

    fn class_declaration(&self, class: &str) -> Option<usize> {
        let wanted = format!("class {class}");
        let text = String::from_utf8_lossy(&self.code);
        text.match_indices(&wanted)
            .map(|(at, _)| at + wanted.len())
            .find(|end| self.ends_word(*end))
    }

    fn ends_word(&self, end: usize) -> bool {
        let next = self.code.get(end).copied().unwrap_or(b' ');
        !next.is_ascii_alphanumeric() && next != b'_'
    }

    fn next_open_brace(&self, from: usize) -> Option<usize> {
        (from..self.code.len()).find(|at| self.code[*at] == Self::OPEN)
    }

    fn closing(depths: &[i32], open: usize) -> Option<usize> {
        let outside = depths[open];
        let after = (open + 1..depths.len()).find(|at| depths[*at] == outside)?;
        Some(after - 1)
    }

    fn headers(&self, body: &Range<usize>) -> Vec<Range<usize>> {
        let level = self.braces[body.start];
        let mut headers = Vec::new();
        let mut start = body.start;
        for end in self.statement_ends(body, level) {
            headers.push(start..end);
            start = self.member_end(end) + 1;
        }
        headers
    }

    fn statement_ends(&self, body: &Range<usize>, level: i32) -> Vec<usize> {
        body.clone()
            .filter(|at| self.ends_statement(*at, level))
            .collect()
    }

    fn ends_statement(&self, at: usize, level: i32) -> bool {
        let ends = matches!(self.code[at], Self::STATEMENT_END | Self::OPEN);
        (self.braces[at], self.parens[at], ends) == (level, 0, true)
    }

    fn member_end(&self, end: usize) -> usize {
        if self.code[end] == Self::OPEN {
            return Self::closing(&self.braces, end).unwrap_or(end);
        }
        end
    }

    fn declared(&self, headers: &[Range<usize>]) -> Result<Vec<JavaMember>, Failure> {
        let mut members = Vec::new();
        for header in headers {
            members.extend(self.member(header)?);
        }
        Ok(members)
    }

    fn member(&self, header: &Range<usize>) -> Result<Option<JavaMember>, Failure> {
        let text = String::from_utf8_lossy(&self.code[header.clone()]);
        JavaDeclaration::parse(&text)
            .member()
            .map_err(|message| self.failure(Self::SHAPE_CODE, message, header))
    }

    fn require_members(
        &self,
        members: Vec<JavaMember>,
        class: &str,
    ) -> Result<Vec<JavaMember>, Failure> {
        if members.is_empty() {
            let message =
                format!("Java web service '{class}' has no field or method the page can use.");
            return Err(self.failure(Self::OWNER_CODE, message, &(0..0)));
        }
        Ok(members)
    }

    fn missing_class(&self, class: &str) -> Failure {
        let message = format!(
            "{} declares no class named '{class}'. A service file holds the class named after it.",
            self.path
        );
        self.failure(Self::OWNER_CODE, message, &(0..0))
    }

    fn failure(&self, code: u16, message: String, at: &Range<usize>) -> Failure {
        let start = at.start
            + self.code[at.clone()]
                .iter()
                .take_while(|byte| byte.is_ascii_whitespace())
                .count();
        Failure::one(diagnostic(
            code,
            message,
            None,
            source_span(self.path, start, at.end.max(start)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::JavaClassReader;
    use crate::native::java_web_services::java_member_kind::JavaMemberKind;

    const PATH: &str = "client/services/ShelfService.java";

    fn read(source: &str) -> Result<Vec<(String, String, Vec<String>, JavaMemberKind)>, String> {
        let members = JavaClassReader::new(PATH, source)
            .members("ShelfService")
            .map_err(|failure| failure.to_string())?;
        Ok(members
            .into_iter()
            .map(|member| {
                (
                    member.name,
                    member.value_type,
                    member.parameters,
                    member.kind,
                )
            })
            .collect())
    }

    #[test]
    fn a_service_class_becomes_fields_methods_and_hooks() {
        let source = "import java.util.*;\n\nclass ShelfService {\n    @LocalState\n    List<String> titles = new ArrayList<>();\n    final String owner = \"{ me; }\";\n    private int hidden = 1;\n    static int shared = 2;\n    Runnable later = () -> { hidden++; };\n    int[] sizes = {1, 2};\n\n    ShelfService() {\n    }\n\n    @OnStart\n    void open() {\n        titles.add(\"a\");\n    }\n\n    @Override\n    public String toString() {\n        return owner;\n    }\n\n    Map<String, Integer> count(final List<String> words, @Deprecated int limit) throws Exception {\n        return new HashMap<>();\n    }\n\n    private record Hidden(int size) {}\n\n    class Inner {\n        int deep = 3;\n    }\n}\n";
        let members = read(source);
        let Err(message) = &members else {
            panic!("Runnable and int[] are not page values: {members:?}");
        };
        assert!(message.contains("later uses Runnable"), "{message}");
        let source = source
            .replace("    Runnable later = () -> { hidden++; };\n", "")
            .replace("    int[] sizes = {1, 2};\n", "");
        let members = read(&source).expect("members");
        let names: Vec<(&str, &str, JavaMemberKind)> = members
            .iter()
            .map(|(name, kind, _, member)| (name.as_str(), kind.as_str(), *member))
            .collect();
        assert_eq!(
            names,
            [
                (
                    "titles",
                    "List<String>",
                    JavaMemberKind::Field { read_only: false }
                ),
                ("owner", "String", JavaMemberKind::Field { read_only: true }),
                ("open", "void", JavaMemberKind::Hook { destroy: false }),
                ("toString", "String", JavaMemberKind::Method),
                ("count", "Map<String, Integer>", JavaMemberKind::Method),
            ]
        );
        assert_eq!(members[4].2, ["List<String>", "int"]);
    }

    #[test]
    fn member_markers_are_blanked_for_the_compiler() {
        let source = "class ShelfService {\n    @SessionState\n    String draft = \"@OnStart\";\n    @OnStop()\n    void close() {}\n}\n";
        let reader = JavaClassReader::new(PATH, source);
        let blanked = reader.without_markers(source);
        assert_eq!(
            blanked,
            "class ShelfService {\n                 \n    String draft = \"@OnStart\";\n             \n    void close() {}\n}\n"
        );
    }

    #[test]
    fn unclear_members_are_refused_with_the_reason() {
        let cases = [
            (
                "    int first = 1, second = 2;\n",
                "Declare one field per statement",
            ),
            (
                "    @LocalState\n    private String theme = \"\";\n",
                "cannot be private or static",
            ),
            (
                "    @OnStart\n    void open(int times) {}\n",
                "takes no arguments",
            ),
            (
                "    @Retry\n    void save() {}\n",
                "@Retry does not work in a Java web service yet",
            ),
            ("    char letter = 'a';\n", "letter uses char"),
        ];
        for (member, reason) in cases {
            let source = format!("class ShelfService {{\n{member}}}\n");
            let message = read(&source).expect_err(reason);
            assert!(message.contains("TY1405"), "{message}");
            assert!(message.contains(reason), "{message}");
        }
    }

    #[test]
    fn a_class_annotation_java_web_services_lack_is_refused() {
        let message = read("import java.util.List;\n\n@Topic(\"news\")\nclass ShelfService {\n    int count = 0;\n}\n")
            .expect_err("topic");
        assert!(
            message.contains("@Topic does not work in a Java web service yet"),
            "{message}"
        );
    }

    #[test]
    fn a_file_without_its_class_or_members_is_refused() {
        let missing = read("class OtherService {\n    int count = 0;\n}\n").expect_err("missing");
        assert!(
            missing.contains("declares no class named 'ShelfService'"),
            "{missing}"
        );
        let empty =
            read("class ShelfService {\n    private int count = 0;\n}\n").expect_err("empty");
        assert!(
            empty.contains("has no field or method the page can use"),
            "{empty}"
        );
    }
}
