use super::injected_field::InjectedField;
use std::path::Path;

pub(crate) struct InjectedContract<'a> {
    language: &'static str,
    marker: &'static str,
    named_marker: &'static str,
    filled: &'static str,
    contracts: &'a [String],
    remotes: &'a [String],
}

impl<'a> InjectedContract<'a> {
    pub(crate) const GO_START: &'static str = "tachyonInject";
    const FORMS: [(&'static str, &'static str, &'static str, &'static str); 7] = [
        (
            "kt",
            "@Needs",
            "@Named(",
            "{indent}private val {name}: {type} = {provider}()",
        ),
        (
            "swift",
            "@Needs",
            "@Named(",
            "{indent}private let {name}: {type} = {provider}()",
        ),
        (
            "java",
            "@Needs",
            "@Named(",
            "{indent}private final {type} {name} = new {provider}();",
        ),
        (
            "cs",
            "[Needs]",
            "[Named(",
            "{indent}private readonly {type} {name} = new {provider}();",
        ),
        (
            "rs",
            "#[Needs]",
            "#[Named(",
            "{indent}{name}: std::sync::Arc<{type}>,",
        ),
        ("go", "// @Needs", "// @Named(", "{indent}{name} *{type}"),
        (
            "rb",
            "#[Needs]",
            "#[Named(",
            "{indent}private def {name}\n{indent}  @{name} ||= {provider}.new\n{indent}end",
        ),
    ];
    pub(super) const MODIFIERS: [&'static str; 9] = [
        "private",
        "public",
        "protected",
        "internal",
        "fileprivate",
        "final",
        "readonly",
        "lateinit",
        "pub",
    ];
    pub(super) const KEYWORDS: [&'static str; 3] = ["val", "var", "let"];
    const EMPTY_ARGUMENTS: &'static str = "()";
    const INJECTED_FOLDERS: [&'static str; 2] = ["client/stores", "client/converters"];
    const REMOTES: &'static str = "client/remotes";
    const HELPERS: &'static str = "client/helpers";
    const HELPER_LANGUAGES: [&'static str; 7] = [
        Self::GO,
        Self::RUBY,
        "swift",
        Self::KOTLIN,
        Self::JAVA,
        "cs",
        "fs",
    ];
    const HELPER_SUFFIX: &'static str = "Helper";
    const KOTLIN: &'static str = "kt";
    const JAVA: &'static str = "java";
    const GO: &'static str = "go";
    const REMOTE_LANGUAGES: [&'static str; 5] = [Self::GO, "kt", "swift", "java", Self::RUBY];
    const RUBY: &'static str = "rb";
    const RUBY_REMOTE: &'static str = "{indent}private def {name}\n{indent}  @{name} ||= {provider}.new(@tachyon_test_context)\n{indent}end";
    const NAMED_QUOTE: char = '"';
    const RUBY_COMMENT: &'static str = "# ";
    const RUBY_COMMENT_START: char = '#';
    const GO_METHOD: &'static str =
        "\nfunc (service *{owner}) {start}() error {\n{assignments}\treturn nil\n}\n";
    const GO_ASSIGNMENT: &'static str = "\t{name}, err := New{provider}()\n\tif err != nil {\n\t\treturn err\n\t}\n\tservice.{name} = {name}\n";
    const GO_HELPER_ASSIGNMENT: &'static str = "\tservice.{name} = &{provider}{}\n";

    pub(crate) fn injected_names<'p>(paths: impl Iterator<Item = &'p Path>) -> Vec<String> {
        paths
            .filter(|path| Self::injected(path))
            .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
            .collect()
    }

    pub(crate) fn helper_names<'p>(paths: impl Iterator<Item = &'p Path>) -> Vec<String> {
        paths
            .filter(|path| Self::injected_helper(path))
            .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
            .collect()
    }

    pub(crate) fn remote_names<'p>(paths: impl Iterator<Item = &'p Path>) -> Vec<String> {
        paths
            .filter(|path| Self::injected_remote(path))
            .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
            .collect()
    }

    pub(crate) fn filled(
        path: &Path,
        source: &str,
        contracts: &'a [String],
        remotes: &'a [String],
    ) -> Option<String> {
        let language = path.extension()?.to_str()?;
        let owner = path.file_stem()?.to_str()?;
        let (form, marker, named_marker, filled) = Self::FORMS
            .iter()
            .find(|(form, _, _, _)| *form == language)?;
        let injected = Self {
            language: form,
            marker,
            named_marker,
            filled,
            contracts,
            remotes,
        };
        injected.rewritten(source, owner)
    }

    fn injected(path: &Path) -> bool {
        let in_folder = Self::INJECTED_FOLDERS
            .iter()
            .any(|folder| path.parent() == Some(Path::new(folder)));
        [
            in_folder,
            Self::injected_remote(path),
            Self::injected_helper(path),
        ]
        .contains(&true)
    }

    pub(crate) fn owned_helper(path: &Path) -> bool {
        [
            path.starts_with(Self::HELPERS),
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| [Self::KOTLIN, Self::JAVA].contains(&extension)),
        ] == [true, true]
    }

    pub(crate) fn injected_helper(path: &Path) -> bool {
        [
            path.starts_with(Self::HELPERS),
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| Self::HELPER_LANGUAGES.contains(&extension)),
        ] == [true, true]
    }

    fn injected_remote(path: &Path) -> bool {
        [
            path.parent() == Some(Path::new(Self::REMOTES)),
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| Self::REMOTE_LANGUAGES.contains(&extension)),
        ] == [true, true]
    }

    fn rewritten(&self, source: &str, owner: &str) -> Option<String> {
        let lines: Vec<&str> = source.split('\n').collect();
        let edits: Vec<(usize, usize, InjectedField)> = (0..lines.len())
            .filter(|at| self.is_marker(lines[*at]))
            .filter_map(|at| self.edit(&lines, at))
            .collect();
        let written: Vec<String> = lines
            .iter()
            .enumerate()
            .map(|(at, line)| self.written_line(at, line, &edits))
            .collect();
        let fields: Vec<&InjectedField> = edits.iter().map(|(_, _, field)| field).collect();
        let start = self.go_start(owner, &fields);
        (!edits.is_empty()).then(|| format!("{}{start}", written.join("\n")))
    }

    fn written_line(
        &self,
        at: usize,
        line: &str,
        edits: &[(usize, usize, InjectedField)],
    ) -> String {
        let marker = edits
            .iter()
            .any(|(marker_at, field_at, _)| self.is_marker_line(at, line, *marker_at, *field_at))
            .then(String::new);
        let field = edits
            .iter()
            .find(|(_, field_at, _)| *field_at == at)
            .map(|(_, _, field)| field.filled(self.form_of(field)));
        field.or(marker).unwrap_or_else(|| line.to_owned())
    }

    fn is_marker_line(&self, at: usize, line: &str, marker_at: usize, field_at: usize) -> bool {
        let between = [at > marker_at, at < field_at] == [true, true];
        let named = [between, self.is_named(line)] == [true, true];
        [at == marker_at, named].contains(&true)
    }

    fn form_of(&self, field: &InjectedField) -> &'static str {
        let ruby_remote = [
            self.language == Self::RUBY,
            self.remotes.contains(&field.provider),
        ] == [true, true];
        [self.filled, Self::RUBY_REMOTE][usize::from(ruby_remote)]
    }

    fn is_marker(&self, line: &str) -> bool {
        let trimmed = line.trim();
        let arguments = trimmed.strip_prefix(self.marker);
        [Some(""), Some(Self::EMPTY_ARGUMENTS)].contains(&arguments)
    }

    fn edit(&self, lines: &[&str], marker_at: usize) -> Option<(usize, usize, InjectedField)> {
        let field_at = (marker_at + 1..lines.len()).find(|at| self.is_field(lines[*at]))?;
        let field = if self.language == Self::RUBY {
            let documentation = Self::ruby_documentation(lines, marker_at, field_at);
            InjectedField::read_ruby(&documentation, lines[field_at])?
        } else {
            InjectedField::read(lines[field_at], self.language == Self::GO)?
        };
        let field = field.named(self.provider(&lines[marker_at + 1..field_at]));
        self.contracts
            .contains(&field.provider)
            .then_some((marker_at, field_at, field))
    }

    fn provider(&self, lines: &[&str]) -> Option<String> {
        let line = lines.iter().find(|line| self.is_named(line))?;
        let arguments = line.trim().strip_prefix(self.named_marker)?;
        let quoted = arguments.split(Self::NAMED_QUOTE).nth(1)?;
        Some(quoted.trim().to_owned())
    }

    fn is_named(&self, line: &str) -> bool {
        line.trim().starts_with(self.named_marker)
    }

    fn ruby_documentation<'l>(
        lines: &[&'l str],
        marker_at: usize,
        field_at: usize,
    ) -> Vec<&'l str> {
        let above = lines[..marker_at]
            .iter()
            .rev()
            .take_while(|line| line.trim().starts_with(Self::RUBY_COMMENT_START));
        let below = lines[marker_at + 1..field_at].iter();
        above.chain(below).copied().collect()
    }

    fn is_field(&self, line: &str) -> bool {
        let trimmed = line.trim();
        let documentation = [
            self.language == Self::RUBY,
            trimmed.starts_with(Self::RUBY_COMMENT),
        ] == [true, true];
        [trimmed.is_empty(), documentation, self.is_named(line)] == [false, false, false]
    }

    fn go_assignment(field: &InjectedField) -> &'static str {
        let helper = field.provider.ends_with(Self::HELPER_SUFFIX);
        [Self::GO_ASSIGNMENT, Self::GO_HELPER_ASSIGNMENT][usize::from(helper)]
    }

    fn go_start(&self, owner: &str, fields: &[&InjectedField]) -> String {
        let assignments: String = fields
            .iter()
            .map(|field| field.filled(Self::go_assignment(field)))
            .collect();
        let wanted = [self.language == Self::GO, !fields.is_empty()] == [true, true];
        if !wanted {
            return String::new();
        }
        Self::GO_METHOD
            .replace("{owner}", owner)
            .replace("{start}", Self::GO_START)
            .replace("{assignments}", &assignments)
    }
}
