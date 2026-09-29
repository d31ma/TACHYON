use super::java_code::JavaCode;
use super::java_declaration::JavaDeclaration;
use super::java_member::JavaMember;
use super::java_value_type::JavaValueType;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::ops::Range;

pub(super) struct JavaClassReader<'a> {
    path: &'a str,
    code: JavaCode,
}

impl<'a> JavaClassReader<'a> {
    const SHAPE_CODE: u16 = 1405;
    const OWNER_CODE: u16 = 1406;
    const CLASS: &'static str = "class";
    const MARKERS: [&'static str; 8] = [
        "@OnStart",
        "@OnStop",
        "@OnEvent",
        "@Order",
        "@InBackground",
        "@Needs",
        "@LocalState",
        "@SessionState",
    ];
    const BLANK: u8 = b' ';
    const HEADER_ENDS: [u8; 2] = *b";}";

    pub(super) fn new(path: &'a str, source: &str) -> Self {
        Self {
            path,
            code: JavaCode::new(source),
        }
    }

    pub(super) fn members(
        &self,
        class: &str,
        values: &JavaValueType,
    ) -> Result<Vec<JavaMember>, Failure> {
        let declared = self
            .code
            .declaration(Self::CLASS, class)
            .ok_or_else(|| self.missing_class(class))?;
        self.check_class_header(declared)?;
        let body = self
            .code
            .body_after(declared)
            .ok_or_else(|| self.missing_class(class))?;
        let headers = self.code.member_headers(&body);
        let members = self.declared(&headers, values)?;
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
        let text = String::from_utf8_lossy(self.code.bytes());
        text.match_indices(marker)
            .map(|(at, _)| at..at + marker.len())
            .filter(|span| self.code.ends_word(span.end))
            .map(|span| self.with_arguments(span))
            .collect()
    }

    fn with_arguments(&self, span: Range<usize>) -> Range<usize> {
        if !self.code.is_open_arguments(span.end) {
            return span;
        }
        let close = self.code.closing_parenthesis(span.end).unwrap_or(span.end);
        span.start..close + 1
    }

    fn blanked(source: &str, spans: &[Range<usize>]) -> String {
        let mut bytes = source.as_bytes().to_vec();
        for span in spans {
            bytes[span.clone()].fill(Self::BLANK);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn check_class_header(&self, declared: usize) -> Result<(), Failure> {
        let header = self.header_start(declared)..declared;
        JavaDeclaration::parse(&self.code.text(&header))
            .refuse_class_annotations()
            .map_err(|message| self.failure(Self::SHAPE_CODE, message, &header))
    }

    fn header_start(&self, declared: usize) -> usize {
        let code = self.code.bytes();
        (0..declared)
            .rev()
            .find(|at| Self::HEADER_ENDS.contains(&code[*at]))
            .map_or(0, |at| at + 1)
    }

    fn declared(
        &self,
        headers: &[Range<usize>],
        values: &JavaValueType,
    ) -> Result<Vec<JavaMember>, Failure> {
        let mut members = Vec::new();
        for header in headers {
            members.extend(self.member(header, values)?);
        }
        Ok(members)
    }

    fn member(
        &self,
        header: &Range<usize>,
        values: &JavaValueType,
    ) -> Result<Option<JavaMember>, Failure> {
        JavaDeclaration::parse(&self.code.text(header))
            .member(values)
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
            + self.code.bytes()[at.clone()]
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
#[allow(clippy::expect_used, clippy::type_complexity)]
mod tests {
    use super::JavaClassReader;
    use crate::native::java_web_services::java_member_kind::JavaMemberKind;
    use crate::native::java_web_services::java_value_type::JavaValueType;

    const PATH: &str = "client/services/ShelfService.java";

    fn read(source: &str) -> Result<Vec<(String, String, Vec<String>, JavaMemberKind)>, String> {
        let members = JavaClassReader::new(PATH, source)
            .members("ShelfService", &JavaValueType::new(Vec::new()))
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
            .map(|(name, kind, _, member)| (name.as_str(), kind.as_str(), member.clone()))
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
    fn listeners_publishers_and_behavior_contexts_stay_off_the_page() {
        let source = "class ShelfService {\n    ApplicationEventPublisher applicationEventPublisher;\n    Behaviors.Context behaviors;\n    int count = 0;\n\n    @OnEvent\n    @Order(-1)\n    void heard(Object value) {}\n}\n";
        let members = read(source).expect("members");
        let names: Vec<&str> = members.iter().map(|(name, ..)| name.as_str()).collect();
        assert_eq!(names, ["count"]);
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
                "    @OnEvent\n    private void heard(Object value) {}\n",
                "cannot be private or static",
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
