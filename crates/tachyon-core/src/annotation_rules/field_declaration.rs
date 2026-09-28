use super::Realm;

pub(super) struct FieldDeclaration {
    modifiers: Vec<String>,
    pub(super) name: String,
    type_text: String,
    optional_name: bool,
    initialized: bool,
    accessors: String,
}

impl FieldDeclaration {
    const TYPE_FIRST_MODIFIERS: [&'static str; 17] = [
        "public",
        "private",
        "protected",
        "internal",
        "readonly",
        "final",
        "static",
        "const",
        "volatile",
        "transient",
        "late",
        "covariant",
        "required",
        "abstract",
        "override",
        "virtual",
        "var",
    ];
    const NOT_FIELD_WORDS: [&'static str; 26] = [
        "class",
        "struct",
        "enum",
        "interface",
        "object",
        "companion",
        "fun",
        "func",
        "fn",
        "def",
        "function",
        "get",
        "set",
        "init",
        "deinit",
        "case",
        "typealias",
        "import",
        "return",
        "static",
        "const",
        "declare",
        "abstract",
        "type",
        "impl",
        "use",
    ];
    const BINDING_WORDS: [&'static str; 6] = ["var", "val", "let", "lateinit", "final", "late"];
    const TAC_BINDINGS: [(&'static str, &'static [&'static str]); 4] = [
        ("dart", &["final", "late"]),
        ("cs", &["required", "readonly"]),
        ("kt", &["val", "lateinit"]),
        ("swift", &["let", "var"]),
    ];
    const SUFFIX_MARKS: [(&'static str, bool, &'static [char]); 8] = [
        ("kt", false, &['?']),
        ("kt", true, &['?']),
        ("dart", false, &['?']),
        ("dart", true, &['?']),
        ("cs", false, &['?']),
        ("cs", true, &['?']),
        ("swift", false, &['?', '!']),
        ("swift", true, &['?']),
    ];
    const NULL_MEMBERS: [(&'static str, &'static [&'static str]); 3] = [
        ("ts", &["null", "undefined"]),
        ("php", &["null"]),
        ("py", &["None"]),
    ];
    const WRAPPERS: [(&'static str, &'static [(&'static str, &'static str)]); 3] = [
        ("py", &[("Optional[", "]"), ("typing.Optional[", "]")]),
        (
            "rs",
            &[
                ("Option<", ">"),
                ("std::option::Option<", ">"),
                ("core::option::Option<", ">"),
            ],
        ),
        ("php", &[("?", "")]),
    ];
    const FILLED_FORMS: [(Realm, bool); 3] =
        [(Realm::Tac, false), (Realm::Tac, true), (Realm::Yon, false)];
    const ANNOTATED_HEADS: [&'static str; 2] = ["Annotated[", "typing.Annotated["];
    const CLASS_VARIABLE: &'static str = "ClassVar[";
    const PUBLIC_WORDS: [&'static str; 2] = ["public", "protected"];
    const NEEDS_TEMPLATES: [(&'static str, &'static str); 2] = [
        ("kt", "{visibility}lateinit var {name}: {type}"),
        ("swift", "{visibility}var {name}: {type}!"),
    ];
    const TEMPLATES: [(&'static str, &'static str); 10] = [
        ("ts", "{modifiers}{name}!: {type}"),
        ("kt", "{visibility}val {name}: {type}"),
        ("swift", "{visibility}let {name}: {type}"),
        ("dart", "{visibility}late final {type} {name}"),
        ("cs", "{modifiers}{type} {name} {accessors}"),
        ("php", "{modifiers}{type} ${name}"),
        ("py", "{name}: {type}"),
        ("rs", "{modifiers}{name}: {type}"),
        ("java", "{modifiers}{type} {name}"),
        ("groovy", "{modifiers}{type} {name}"),
    ];

    pub(super) fn read(language: &str, text: &str) -> Option<Self> {
        let declared_end = Self::assignment(text).unwrap_or(text.len());
        let (declared, accessors) = Self::accessors(language, text[..declared_end].trim_end());
        let (before, written_name, type_text) = Self::parts(language, declared)?;
        let field = Self {
            modifiers: before.split_whitespace().map(str::to_owned).collect(),
            name: written_name
                .trim_start_matches('$')
                .trim_end_matches(['?', '!'])
                .to_owned(),
            optional_name: written_name.ends_with('?'),
            type_text,
            initialized: Self::initialized(language, text, declared_end),
            accessors,
        };
        field.names_a_field().then_some(field)
    }

    pub(super) fn type_text(&self) -> &str {
        &self.type_text
    }

    pub(super) fn is_public_member(&self) -> bool {
        let property = !self.accessors.is_empty();
        let public = self
            .modifiers
            .iter()
            .any(|word| Self::PUBLIC_WORDS.contains(&word.as_str()));
        [property, public].contains(&true)
    }

    pub(super) fn is_filled(&self, language: &str, realm: Realm) -> bool {
        self.is_bound(Self::binding_words(language, realm), realm)
    }

    pub(super) fn is_nullable(&self, language: &str, needs: bool) -> bool {
        self.differs_from(&self.bare_type(language, needs))
    }

    pub(super) fn filled_form(&self, language: &str, needs: bool) -> String {
        let modifiers = Self::spaced(self.modifiers.iter());
        let visibility = Self::spaced(
            self.modifiers
                .iter()
                .filter(|word| !Self::BINDING_WORDS.contains(&word.as_str())),
        );
        Self::template(language, needs)
            .replace("{modifiers}", &modifiers)
            .replace("{visibility}", &visibility)
            .replace("{name}", &self.name)
            .replace("{type}", &self.bare_type(language, needs))
            .replace("{accessors}", &self.accessors)
            .trim_end()
            .to_owned()
    }

    fn bare_type(&self, language: &str, needs: bool) -> String {
        let (head, core, tail) = Self::annotated(&self.type_text);
        let core = Self::without_suffix(core, Self::suffix_marks(language, needs));
        let core = Self::without_nulls(&core, Self::null_members(language));
        let core = Self::without_wrapper(&core, Self::wrappers(language));
        format!("{head}{core}{tail}")
    }

    fn differs_from(&self, bare: &str) -> bool {
        self.optional_name || bare != self.type_text
    }

    fn is_bound(&self, words: Option<&[&str]>, realm: Realm) -> bool {
        !self.initialized
            && self.is_filled_form(realm)
            && words.is_none_or(|words| {
                self.modifiers
                    .iter()
                    .any(|word| words.contains(&word.as_str()))
            })
    }

    fn is_filled_form(&self, realm: Realm) -> bool {
        Self::FILLED_FORMS.contains(&(realm, !self.accessors.is_empty()))
    }

    fn names_a_field(&self) -> bool {
        self.name
            .chars()
            .next()
            .is_some_and(|first| !first.is_ascii_digit())
            && self.name.chars().all(Self::is_word_character)
            && !self
                .modifiers
                .iter()
                .any(|word| Self::NOT_FIELD_WORDS.contains(&word.as_str()))
            && !self.type_text.contains(['{', '='])
            && !self.type_text.contains(Self::CLASS_VARIABLE)
    }

    fn is_word_character(character: char) -> bool {
        character.is_alphanumeric() || character == '_'
    }

    fn initialized(language: &str, text: &str, declared_end: usize) -> bool {
        declared_end < text.len()
            || language == "kt" && text.split_whitespace().any(|word| word == "by")
    }

    fn accessors<'a>(language: &str, declared: &'a str) -> (&'a str, String) {
        match (language, declared.find('{')) {
            ("cs", Some(open)) => (declared[..open].trim_end(), declared[open..].to_owned()),
            _ => (declared, String::new()),
        }
    }

    fn parts(language: &str, declared: &str) -> Option<(String, String, String)> {
        match language {
            "ts" | "kt" | "swift" | "rs" | "py" => Self::colon_parts(declared),
            "dart" | "cs" | "java" | "groovy" | "php" => Self::type_first_parts(declared),
            _ => None,
        }
    }

    fn colon_parts(declared: &str) -> Option<(String, String, String)> {
        let (prefix, type_text) = declared.split_once(':')?;
        let prefix = prefix.trim_end();
        let name = prefix.split_whitespace().next_back()?;
        let before = &prefix[..prefix.len() - name.len()];
        (!prefix.contains('(')).then(|| {
            (
                before.to_owned(),
                name.to_owned(),
                type_text.trim().to_owned(),
            )
        })
    }

    fn type_first_parts(declared: &str) -> Option<(String, String, String)> {
        let name = declared.split_whitespace().next_back()?;
        let before = declared[..declared.len() - name.len()]
            .split_whitespace()
            .collect::<Vec<_>>();
        let modifiers = before
            .iter()
            .take_while(|word| Self::TYPE_FIRST_MODIFIERS.contains(word))
            .count();
        Some((
            before[..modifiers].join(" "),
            name.to_owned(),
            before[modifiers..].join(" "),
        ))
    }

    fn assignment(text: &str) -> Option<usize> {
        let bytes = text.as_bytes();
        (0..bytes.len()).find(|at| Self::is_assignment_at(bytes, *at))
    }

    fn is_assignment_at(bytes: &[u8], at: usize) -> bool {
        let next = bytes.get(at + 1).copied().unwrap_or(b' ');
        let previous = at
            .checked_sub(1)
            .and_then(|before| bytes.get(before))
            .copied()
            .unwrap_or(b' ');
        bytes[at] == b'=' && !b">=".contains(&next) && !b"=!<>".contains(&previous)
    }

    fn binding_words(language: &str, realm: Realm) -> Option<&'static [&'static str]> {
        Self::TAC_BINDINGS
            .iter()
            .find(|(name, _)| (*name, Realm::Tac) == (language, realm))
            .map(|(_, words)| *words)
    }

    fn suffix_marks(language: &str, needs: bool) -> &'static [char] {
        Self::SUFFIX_MARKS
            .iter()
            .find(|(name, marked, _)| (*name, *marked) == (language, needs))
            .map_or(&[], |(_, _, marks)| *marks)
    }

    fn null_members(language: &str) -> &'static [&'static str] {
        Self::NULL_MEMBERS
            .iter()
            .find(|(name, _)| *name == language)
            .map_or(&[], |(_, members)| *members)
    }

    fn wrappers(language: &str) -> &'static [(&'static str, &'static str)] {
        Self::WRAPPERS
            .iter()
            .find(|(name, _)| *name == language)
            .map_or(&[], |(_, wrappers)| *wrappers)
    }

    fn template(language: &str, needs: bool) -> &'static str {
        Self::NEEDS_TEMPLATES
            .iter()
            .find(|(name, _)| (*name, true) == (language, needs))
            .or_else(|| Self::TEMPLATES.iter().find(|(name, _)| *name == language))
            .map_or("{modifiers}{name}: {type}", |(_, template)| *template)
    }

    fn annotated(type_text: &str) -> (&str, &str, &str) {
        let head = Self::ANNOTATED_HEADS
            .iter()
            .copied()
            .find(|head| type_text.starts_with(head))
            .unwrap_or("");
        let inner = &type_text[head.len()..];
        let core_end = Self::core_length(inner, !head.is_empty());
        (head, &inner[..core_end], &inner[core_end..])
    }

    fn core_length(inner: &str, annotated: bool) -> usize {
        if annotated {
            return crate::go_source::split_top_level(inner, b',')[0].len();
        }
        inner.len()
    }

    fn without_suffix(text: &str, marks: &[char]) -> String {
        text.strip_suffix(marks).unwrap_or(text).to_owned()
    }

    fn without_nulls(text: &str, nulls: &[&str]) -> String {
        let members = crate::go_source::split_top_level(text, b'|');
        let kept = members
            .iter()
            .map(|member| member.trim())
            .filter(|member| !nulls.contains(member))
            .collect::<Vec<_>>();
        Self::joined_members(text, members.len(), &kept)
    }

    fn joined_members(text: &str, count: usize, kept: &[&str]) -> String {
        if kept.len() == count {
            return text.to_owned();
        }
        kept.join(" | ")
    }

    fn without_wrapper(text: &str, wrappers: &[(&str, &str)]) -> String {
        wrappers
            .iter()
            .find_map(|(open, close)| text.strip_prefix(open)?.strip_suffix(close))
            .map_or(text, str::trim)
            .to_owned()
    }

    fn spaced<'a>(words: impl Iterator<Item = &'a String>) -> String {
        words.map(|word| word.clone() + " ").collect()
    }
}
