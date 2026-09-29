use super::store_field::StoreField;
use std::path::Path;

pub(crate) struct InjectedStore<'a> {
    language: &'static str,
    marker: &'static str,
    filled: &'static str,
    stores: &'a [String],
}

impl<'a> InjectedStore<'a> {
    pub(crate) const GO_START: &'static str = "tachyonStores";
    const FORMS: [(&'static str, &'static str, &'static str); 7] = [
        (
            "kt",
            "@Needs",
            "{indent}private val {name}: {type} = {type}()",
        ),
        (
            "swift",
            "@Needs",
            "{indent}private let {name}: {type} = {type}()",
        ),
        (
            "java",
            "@Needs",
            "{indent}private final {type} {name} = new {type}();",
        ),
        (
            "cs",
            "[Needs]",
            "{indent}private readonly {type} {name} = new {type}();",
        ),
        ("rs", "#[Needs]", "{indent}{name}: std::sync::Arc<{type}>,"),
        ("go", "// @Needs", "{indent}{name} *{type}"),
        (
            "rb",
            "#[Needs]",
            "{indent}private def {name}\n{indent}  @{name} ||= {type}.new\n{indent}end",
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
    const STORE_FOLDER: &'static str = "client/stores";
    const GO: &'static str = "go";
    const RUBY: &'static str = "rb";
    const RUBY_COMMENT: &'static str = "# ";
    const GO_METHOD: &'static str =
        "\nfunc (service *{owner}) {start}() error {\n{assignments}\treturn nil\n}\n";
    const GO_ASSIGNMENT: &'static str = "\t{name}, err := New{type}()\n\tif err != nil {\n\t\treturn err\n\t}\n\tservice.{name} = {name}\n";

    pub(crate) fn store_names<'p>(paths: impl Iterator<Item = &'p Path>) -> Vec<String> {
        paths
            .filter(|path| path.parent() == Some(Path::new(Self::STORE_FOLDER)))
            .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
            .collect()
    }

    pub(crate) fn filled(path: &Path, source: &str, stores: &'a [String]) -> Option<String> {
        let language = path.extension()?.to_str()?;
        let owner = path.file_stem()?.to_str()?;
        let (form, marker, filled) = Self::FORMS.iter().find(|(form, _, _)| *form == language)?;
        let injected = Self {
            language: form,
            marker,
            filled,
            stores,
        };
        let mut lines: Vec<String> = source.split('\n').map(str::to_owned).collect();
        let marked: Vec<usize> = (0..lines.len())
            .filter(|at| injected.is_marker(&lines[*at]))
            .collect();
        let edits: Vec<(usize, usize, StoreField)> = marked
            .into_iter()
            .filter_map(|at| injected.edit(&lines, at))
            .collect();
        for (marker_at, field_at, field) in &edits {
            lines[*marker_at] = String::new();
            lines[*field_at] = field.filled(injected.filled);
        }
        let fields: Vec<&StoreField> = edits.iter().map(|(_, _, field)| field).collect();
        let start = injected.go_start(owner, &fields);
        (!edits.is_empty()).then(|| format!("{}{start}", lines.join("\n")))
    }

    fn is_marker(&self, line: &str) -> bool {
        let trimmed = line.trim();
        let arguments = trimmed.strip_prefix(self.marker);
        [Some(""), Some(Self::EMPTY_ARGUMENTS)].contains(&arguments)
    }

    fn edit(&self, lines: &[String], marker_at: usize) -> Option<(usize, usize, StoreField)> {
        let field_at = (marker_at + 1..lines.len()).find(|at| self.is_field(&lines[*at]))?;
        let field = if self.language == Self::RUBY {
            StoreField::read_ruby(&lines[marker_at + 1..field_at], &lines[field_at])?
        } else {
            StoreField::read(&lines[field_at], self.language == Self::GO)?
        };
        self.stores
            .contains(&field.type_name)
            .then_some((marker_at, field_at, field))
    }

    fn is_field(&self, line: &str) -> bool {
        let trimmed = line.trim();
        let documentation = [
            self.language == Self::RUBY,
            trimmed.starts_with(Self::RUBY_COMMENT),
        ] == [true, true];
        [trimmed.is_empty(), documentation] == [false, false]
    }

    fn go_start(&self, owner: &str, fields: &[&StoreField]) -> String {
        let assignments: String = fields
            .iter()
            .map(|field| field.filled(Self::GO_ASSIGNMENT))
            .collect();
        let wanted = [self.language == Self::GO, !fields.is_empty()] == [true, true];
        let method = Self::GO_METHOD
            .replace("{owner}", owner)
            .replace("{start}", Self::GO_START)
            .replace("{assignments}", &assignments);
        wanted.then_some(method).unwrap_or_default()
    }
}
