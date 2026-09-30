use super::field_name::FieldName;
use super::injected_field::InjectedField;
use super::nullable_field::NullableField;
use super::{AnnotationRules, Arguments, Realm, Rule, Target};
use crate::template::required_fields::RequiredFields;
use std::ops::Range;

struct Found {
    rule: &'static Rule,
    at: usize,
    target: Option<Target>,
    has_arguments: bool,
    is_static: bool,
    declaration: usize,
    method: Option<usize>,
    inline: bool,
}

struct Written {
    name: String,
    end: usize,
    has_arguments: bool,
}

pub(super) struct Scan {
    language: String,
    spelling: String,
    code: String,
    found: Vec<Found>,
}

impl Scan {
    const MAX_MARKERS: usize = 4096;
    const SEPARATORS: [(&'static str, &'static [u8]); 1] = [("rs", b",;\n")];
    const DEFAULT_SEPARATORS: &'static [u8] = b";\n";
    const STATIC_DECORATORS: [&'static str; 2] = ["staticmethod", "classmethod"];
    const TYPE_WORDS: [&'static str; 10] = [
        "class",
        "struct",
        "object",
        "record",
        "interface",
        "protocol",
        "trait",
        "enum",
        "actor",
        "module",
    ];
    const METHOD_WORDS: [&'static str; 8] = [
        "def",
        "fn",
        "func",
        "fun",
        "function",
        "constructor",
        "init",
        "deinit",
    ];
    const FIELD_WORDS: [&'static str; 7] = [
        "let",
        "var",
        "val",
        "const",
        "attr_reader",
        "attr_accessor",
        "attr_writer",
    ];
    const PROPERTY_WORDS: [&'static str; 6] =
        ["val", "var", "private", "public", "protected", "readonly"];
    const MODIFIERS: [&'static str; 30] = [
        "public",
        "private",
        "protected",
        "internal",
        "fileprivate",
        "final",
        "open",
        "abstract",
        "sealed",
        "static",
        "override",
        "async",
        "export",
        "default",
        "readonly",
        "pub",
        "unsafe",
        "suspend",
        "inline",
        "data",
        "lateinit",
        "late",
        "external",
        "virtual",
        "required",
        "mutating",
        "nonisolated",
        "declare",
        "companion",
        "partial",
    ];

    pub(super) fn run(extension: &str, source: &str) -> Self {
        Self::run_as(extension, extension, source)
    }

    pub(super) fn run_as(extension: &str, spelling: &str, source: &str) -> Self {
        let mut scan = Self {
            language: extension.to_owned(),
            spelling: spelling.to_owned(),
            code: String::new(),
            found: Vec::new(),
        };
        let mask = crate::lexical::code_mask(extension, source);
        if mask.error.is_some() {
            return scan;
        }
        let code: Vec<u8> = source
            .bytes()
            .zip(mask.code.iter())
            .map(|(byte, visible)| {
                if *visible || byte == b'\n' {
                    byte
                } else {
                    b' '
                }
            })
            .collect();
        scan.code = String::from_utf8_lossy(&code).into_owned();
        let mut opens: Vec<usize> = Vec::new();
        let mut boundary = true;
        let mut at = if source.starts_with('\u{feff}') { 3 } else { 0 };
        while at < code.len() && scan.found.len() < Self::MAX_MARKERS {
            let byte = code[at];
            if byte == b'\n' {
                boundary = true;
                at += 1;
                continue;
            }
            if byte.is_ascii_whitespace() {
                at += 1;
                continue;
            }
            if boundary && let Some(written) = scan.written_at(&code, source, at) {
                if let Some(rule) = scan.rule(&written.name) {
                    let found =
                        scan.locate(&code, source, at, &written, rule, opens.last().copied());
                    scan.found.push(found);
                }
                at = written.end;
                boundary = true;
                continue;
            }
            match byte {
                b'(' => opens.push(at),
                b')' => {
                    opens.pop();
                }
                _ => {}
            }
            boundary = matches!(byte, b'(' | b',');
            at += 1;
        }
        if scan.language == "py" {
            scan.python_annotated(source);
        }
        scan
    }

    fn python_annotated(&mut self, source: &str) {
        let found =
            crate::python_parameters::PythonAnnotated::markers(&self.code, source, |name| {
                self.rule(name).is_some()
            });
        for marker in found.markers {
            let Some(rule) = self.rule(&marker.written) else {
                continue;
            };
            let target = if marker.parameters.is_some() {
                Target::Parameter
            } else {
                Target::Field
            };
            self.found.push(Found {
                rule,
                at: marker.at,
                target: Some(target),
                has_arguments: !marker.arguments.is_empty(),
                is_static: false,
                declaration: marker.declaration,
                method: marker.parameters,
                inline: false,
            });
        }
        self.found.truncate(Self::MAX_MARKERS);
        self.found.sort_by_key(|found| found.at);
    }

    fn prefix(&self) -> &'static [u8] {
        match self.language.as_str() {
            "rs" | "php" | "rb" => b"#[",
            "cs" => b"[",
            _ => b"@",
        }
    }

    fn written_at(&self, code: &[u8], source: &str, at: usize) -> Option<Written> {
        let prefix = self.prefix();
        if !code[at..].starts_with(prefix) {
            return None;
        }
        let name_start = at + prefix.len();
        let name_length = code[name_start..]
            .iter()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'.'))
            .count();
        if !code.get(name_start).is_some_and(u8::is_ascii_alphabetic) {
            return None;
        }
        let name = std::str::from_utf8(&code[name_start..name_start + name_length])
            .ok()?
            .to_owned();
        let mut end = name_start + name_length;
        let mut has_arguments = false;
        if code.get(end) == Some(&b'(') {
            let close = Self::closing(code, end)?;
            has_arguments = !source.get(end + 1..close)?.trim().is_empty();
            end = close + 1;
        }
        if prefix != b"@" {
            while code.get(end) == Some(&b' ') {
                end += 1;
            }
            if code.get(end) != Some(&b']') {
                return None;
            }
            end += 1;
        }
        Some(Written {
            name,
            end,
            has_arguments,
        })
    }

    fn closing(code: &[u8], open: usize) -> Option<usize> {
        let mut depth = 0_usize;
        for (offset, byte) in code[open..].iter().enumerate() {
            match byte {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(open + offset);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn rule(&self, written: &str) -> Option<&'static Rule> {
        if written.contains('.') {
            return None;
        }
        let (namespace, plain) = match written.rsplit_once("::") {
            Some((namespace, plain)) => (Some(namespace), plain),
            None => (None, written),
        };
        if namespace.is_some_and(|namespace| !matches!(namespace, "yon" | "tac")) {
            return None;
        }
        if self.language == "rs" && matches!(plain, "test" | "allow") {
            return None;
        }
        if matches!(self.language.as_str(), "js" | "ts" | "go" | "rb") {
            return AnnotationRules::rules().find(|rule| rule.name == plain);
        }
        AnnotationRules::find(plain)
    }

    fn locate(
        &self,
        code: &[u8],
        source: &str,
        at: usize,
        written: &Written,
        rule: &'static Rule,
        open: Option<usize>,
    ) -> Found {
        let (declaration, decorated_static) = self.declaration_start(code, source, written.end);
        let line_end = code[declaration..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(code.len(), |offset| declaration + offset);
        let text = std::str::from_utf8(&code[declaration..line_end]).unwrap_or_default();
        let (mut target, is_static) = self.classify(text);
        if open.is_some() {
            target = Some(Target::Parameter);
            if Self::is_property_parameter(text) {
                target = Some(Target::Field);
            }
        }
        Found {
            rule,
            at,
            target,
            has_arguments: written.has_arguments,
            is_static: is_static || decorated_static,
            declaration,
            method: open,
            inline: true,
        }
    }

    fn declaration_start(&self, code: &[u8], source: &str, mut at: usize) -> (usize, bool) {
        let mut decorated_static = false;
        loop {
            while code.get(at).is_some_and(u8::is_ascii_whitespace) {
                at += 1;
            }
            let Some(written) = self.written_at(code, source, at) else {
                return (at, decorated_static);
            };
            decorated_static |= Self::STATIC_DECORATORS.contains(&written.name.as_str());
            at = written.end;
        }
    }

    fn is_property_parameter(text: &str) -> bool {
        let word = text
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .next()
            .unwrap_or("");
        Self::PROPERTY_WORDS.contains(&word)
    }

    fn classify(&self, text: &str) -> (Option<Target>, bool) {
        let mut rest = text.trim();
        let mut is_static = false;
        if rest.is_empty() || rest.starts_with('}') {
            return (None, false);
        }
        while let Some((word, after)) = Self::first_word(rest) {
            let names_a_method = after.starts_with('(') && word != "pub";
            if !Self::MODIFIERS.contains(&word) || after.is_empty() || names_a_method {
                break;
            }
            is_static |= word == "static";
            rest = if word == "pub" {
                Self::after_visibility_scope(after)
            } else {
                after
            };
        }
        if let Some((word, after)) = Self::first_word(rest) {
            let rust_impl = self.language == "rs" && word == "impl";
            if rust_impl || self.is_type_word(word) && Self::names_something(rest, word, after) {
                return (Some(Target::Type), is_static);
            }
            if Self::METHOD_WORDS.contains(&word) {
                let ruby_static = self.language == "rb" && after.starts_with("self.");
                let rust_static = self.language == "rs" && Self::rust_static(after);
                return (
                    Some(Target::Method),
                    is_static || ruby_static || rust_static,
                );
            }
            if Self::FIELD_WORDS.contains(&word) {
                return (Some(Target::Field), is_static);
            }
        }
        let bytes = rest.as_bytes();
        let mut generics = 0_usize;
        for (index, byte) in bytes.iter().enumerate() {
            let arrow = index > 0 && matches!(bytes[index - 1], b'=' | b'-');
            match byte {
                b'<' => generics += 1,
                b'>' if generics > 0 && !arrow => generics -= 1,
                _ if generics > 0 => {}
                b'(' => return (Some(Target::Method), is_static),
                b'=' | b';' | b':' | b'{' | b',' => return (Some(Target::Field), is_static),
                _ => {}
            }
        }
        (Some(Target::Field), is_static)
    }

    fn is_type_word(&self, word: &str) -> bool {
        match (self.language.as_str(), word) {
            ("go", "type") | ("kt", "object") => true,
            (_, "object") => false,
            _ => Self::TYPE_WORDS.contains(&word),
        }
    }

    fn names_something(rest: &str, word: &str, after: &str) -> bool {
        let spaced = rest[word.len()..].starts_with(char::is_whitespace);
        let named = after.starts_with(|character: char| {
            character.is_alphanumeric() || matches!(character, '_' | '<' | '$')
        });
        spaced && named
    }

    fn has_constructor(&self, declaration: usize) -> bool {
        let header_end = self.code[declaration..]
            .find('\n')
            .map_or(self.code.len(), |offset| declaration + offset);
        let header = &self.code[declaration..header_end];
        let name = Self::declared_name(header);
        if self.language == "kt" && Self::has_primary_constructor(header, name) {
            return true;
        }
        self.class_bodies(declaration, header_end, name)
            .into_iter()
            .flat_map(|body| self.code[body].lines())
            .any(|line| self.starts_constructor(Self::without_modifiers(line.trim()), name))
    }

    fn class_bodies(&self, declaration: usize, header_end: usize, name: &str) -> Vec<Range<usize>> {
        match self.language.as_str() {
            "py" | "rb" => vec![self.indented_body(declaration, header_end)],
            "rs" => self.rust_impl_bodies(name),
            _ => self.code[declaration..]
                .find('{')
                .and_then(|offset| self.brace_body(declaration + offset))
                .into_iter()
                .collect(),
        }
    }

    fn indented_body(&self, declaration: usize, header_end: usize) -> Range<usize> {
        let line_start = self.code[..declaration].rfind('\n').map_or(0, |at| at + 1);
        let indent = declaration - line_start;
        let mut end = header_end;
        for line in self.code[header_end..].split_inclusive('\n').skip(1) {
            let text = line.trim_end();
            let depth = text.len() - text.trim_start().len();
            if !text.trim().is_empty() && depth <= indent {
                break;
            }
            end += line.len();
        }
        header_end..(end + 1).min(self.code.len())
    }

    fn brace_body(&self, open: usize) -> Option<Range<usize>> {
        let mut depth = 0_usize;
        for (offset, byte) in self.code.as_bytes()[open..].iter().enumerate() {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(open + 1..open + offset);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn rust_impl_bodies(&self, name: &str) -> Vec<Range<usize>> {
        let mut bodies = Vec::new();
        for (at, _) in self.code.match_indices("impl") {
            let Some(open) = self.code[at..].find('{').map(|offset| at + offset) else {
                continue;
            };
            let header = &self.code[at..open];
            let names_type = header
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .any(|word| word == name);
            if names_type && let Some(body) = self.brace_body(open) {
                bodies.push(body);
            }
        }
        bodies
    }

    fn declared_name(header: &str) -> &str {
        let mut rest = Self::without_modifiers(header.trim());
        if let Some((_, after)) = Self::first_word(rest) {
            rest = after;
        }
        let length = rest
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        &rest[..length]
    }

    fn has_primary_constructor(header: &str, name: &str) -> bool {
        let Some(at) = header.find(&format!("{name}(")) else {
            return false;
        };
        let inside = &header[at + name.len() + 1..];
        let close = inside.find(')').unwrap_or(inside.len());
        !name.is_empty() && !inside[..close].trim().is_empty()
    }

    fn without_modifiers(mut text: &str) -> &str {
        while let Some((word, after)) = Self::first_word(text) {
            let modifier = Self::MODIFIERS.contains(&word)
                || matches!(word, "convenience" | "factory" | "const");
            if !modifier || after.starts_with('(') && word != "pub" {
                break;
            }
            text = if word == "pub" {
                Self::after_visibility_scope(after)
            } else {
                after
            };
        }
        text
    }

    fn starts_constructor(&self, line: &str, name: &str) -> bool {
        let after_name = if name.is_empty() {
            None
        } else {
            line.strip_prefix(name)
        };
        let named = after_name.is_some_and(|rest| rest.starts_with('('));
        let dart_named = after_name.is_some_and(|rest| rest.starts_with('.'));
        match self.language.as_str() {
            "js" | "ts" => line.starts_with("constructor(") || line.starts_with("constructor ("),
            "py" => line.starts_with("def __init__("),
            "php" => line.starts_with("function __construct("),
            "rb" => {
                line == "def initialize"
                    || line.starts_with("def initialize(")
                    || line.starts_with("def initialize ")
            }
            "swift" => line.starts_with("init(") || line.starts_with("init?("),
            "rs" => line.starts_with("fn new("),
            "kt" => line.starts_with("constructor("),
            "java" | "groovy" | "cs" => named,
            "dart" => named || dart_named,
            _ => false,
        }
    }

    fn first_word(text: &str) -> Option<(&str, &str)> {
        let length = text
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        if length == 0 {
            return None;
        }
        Some((&text[..length], text[length..].trim_start()))
    }

    fn after_visibility_scope(text: &str) -> &str {
        match text.find(')') {
            Some(close) if text.starts_with('(') && close < 16 => text[close + 1..].trim_start(),
            _ => text,
        }
    }

    fn rust_static(after: &str) -> bool {
        let Some(open) = after.find('(') else {
            return false;
        };
        let Some(close) = after[open..].find(')') else {
            return false;
        };
        !after[open..open + close].contains("self")
    }

    pub(super) fn problem(&self, realm: Option<Realm>) -> Option<(usize, &'static Rule, String)> {
        let class_names: Vec<&str> = self
            .found
            .iter()
            .filter(|found| found.target == Some(Target::Type))
            .map(|found| found.rule.name)
            .collect();
        for (index, found) in self.found.iter().enumerate() {
            if let Some(message) = self.problem_of(index, found, realm, &class_names) {
                return Some((found.at, found.rule, message));
            }
        }
        None
    }

    fn problem_of(
        &self,
        index: usize,
        found: &Found,
        realm: Option<Realm>,
        class_names: &[&str],
    ) -> Option<String> {
        let rule = found.rule;
        let language = self.spelling.as_str();
        if rule.is_retired() {
            return Some(Messages::retired(rule, language));
        }
        if let Some(realm) = realm
            && realm.languages().contains(&language)
            && !rule.allows(language, realm)
        {
            return Some(Messages::unavailable(rule, language, realm));
        }
        if let Some(target) = found.target
            && !rule.targets.contains(&target)
        {
            return Some(Messages::wrong_target(rule, language, target));
        }
        let python_decorated = language == "py" && found.inline && rule.python_annotated();
        if python_decorated && let Some(target @ (Target::Field | Target::Parameter)) = found.target
        {
            return Some(Messages::python_annotated(rule, target));
        }
        match rule.arguments {
            Arguments::None if found.has_arguments => {
                return Some(Messages::no_arguments(rule, language));
            }
            Arguments::Required if !found.has_arguments => {
                return Some(Messages::needs_arguments(rule, language));
            }
            _ => {}
        }
        let target = found.target?;
        if rule.refuses_constructor
            && target == Target::Type
            && self.has_constructor(found.declaration)
        {
            return Some(Messages::constructor(rule, language));
        }
        if rule.instance && found.is_static {
            return Some(Messages::not_static(rule, language, target));
        }
        self.neighbour_problem(index, found, target)
            .or_else(|| Self::class_problem(found, language, class_names))
    }

    pub(super) fn nullable_problem(
        &self,
        source: &str,
        realm: Option<Realm>,
    ) -> Option<(usize, u16, String)> {
        self.nullable_fields(source, realm)
            .into_iter()
            .next()
            .map(|field| {
                (
                    field.range.start,
                    NullableField::CODE,
                    field.message(&self.language),
                )
            })
    }

    pub(super) fn nullable_fields(&self, source: &str, realm: Option<Realm>) -> Vec<NullableField> {
        let needs = self.needs_fields();
        realm
            .map(|realm| {
                self.found
                    .iter()
                    .filter(|found| found.rule.refuses_nullable)
                    .flat_map(|found| self.nullable_of(found, source, (realm, &needs)))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn injected_problem(
        &self,
        source: &str,
        realm: Option<Realm>,
    ) -> Option<(usize, u16, String)> {
        self.injected_fields(source, realm)
            .into_iter()
            .next()
            .map(|field| (field.at, InjectedField::CODE, field.message(&self.language)))
    }

    pub(super) fn injected_fields(&self, source: &str, realm: Option<Realm>) -> Vec<InjectedField> {
        let needs = self.needs_fields();
        realm
            .map(|realm| {
                self.found
                    .iter()
                    .flat_map(|found| self.injected_of(found, source, (realm, &needs)))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn injected_of(
        &self,
        found: &Found,
        source: &str,
        context: (Realm, &[usize]),
    ) -> Vec<InjectedField> {
        let (realm, needs) = context;
        match (found.target, found.rule.name) {
            (Some(Target::Type), "NeedsAll") => {
                let members = self.class_members(found.declaration, source, needs);
                InjectedField::in_class(&self.language, realm, source, members)
            }
            (Some(Target::Field), "Needs") => {
                let range = self.field_range(found.declaration, source);
                InjectedField::on_field(&self.language, realm, source, range)
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn framework_members(&self, source: &str, realm: Option<Realm>) -> Vec<usize> {
        let mut members = self.marked_members();
        members.extend(
            realm
                .map(|realm| self.filled_fields(source, realm))
                .unwrap_or_default(),
        );
        members
    }

    fn marked_members(&self) -> Vec<usize> {
        self.found
            .iter()
            .filter(|found| {
                matches!(
                    found.target,
                    Some(Target::Method | Target::Field | Target::Parameter)
                )
            })
            .map(|found| found.method.unwrap_or(found.declaration))
            .collect()
    }

    fn filled_fields(&self, source: &str, realm: Realm) -> Vec<usize> {
        let needs = self.needs_fields();
        self.found
            .iter()
            .filter(|found| (found.rule.name, found.target) == ("NeedsAll", Some(Target::Type)))
            .flat_map(|found| self.class_members(found.declaration, source, &needs))
            .flat_map(|range| FieldName::read(&self.language, realm, source, range))
            .filter(|field| field.filled)
            .map(|field| field.at)
            .collect()
    }

    fn needs_fields(&self) -> Vec<usize> {
        self.found
            .iter()
            .filter(|found| (found.rule.name, found.target) == ("Needs", Some(Target::Field)))
            .map(|found| found.declaration)
            .collect()
    }

    fn nullable_of(
        &self,
        found: &Found,
        source: &str,
        context: (Realm, &[usize]),
    ) -> Vec<NullableField> {
        match (found.target, found.rule.name) {
            (Some(Target::Type), "NeedsAll") => {
                self.nullable_class(found.declaration, source, context)
            }
            (Some(Target::Field), _) => self.nullable_field(found.declaration, source),
            _ => Vec::new(),
        }
    }

    fn nullable_field(&self, declaration: usize, source: &str) -> Vec<NullableField> {
        let range = self.field_range(declaration, source);
        NullableField::on_field(&self.language, source, range)
            .into_iter()
            .collect()
    }

    fn field_range(&self, declaration: usize, source: &str) -> Range<usize> {
        let piece = self
            .pieces(declaration..self.code.len())
            .into_iter()
            .next()
            .unwrap_or(declaration..declaration);
        self.declaration_range(source, self.trimmed(piece))
    }

    fn nullable_class(
        &self,
        declaration: usize,
        source: &str,
        context: (Realm, &[usize]),
    ) -> Vec<NullableField> {
        let (realm, needs) = context;
        let declarations = self.class_members(declaration, source, needs);
        NullableField::in_class(&self.language, realm, source, declarations)
    }

    fn class_members(
        &self,
        declaration: usize,
        source: &str,
        needs: &[usize],
    ) -> Vec<Range<usize>> {
        let header_end = self.code[declaration..]
            .find('\n')
            .map_or(self.code.len(), |offset| declaration + offset);
        let name = Self::declared_name(&self.code[declaration..header_end]);
        let body = self
            .brace_body_after(declaration)
            .filter(|_| self.language == "rs")
            .or_else(|| {
                self.class_bodies(declaration, header_end, name)
                    .into_iter()
                    .next()
            })
            .unwrap_or(declaration..declaration);
        let pieces = self
            .pieces(body)
            .into_iter()
            .map(|piece| self.trimmed(piece))
            .collect();
        self.unless_defaulted(self.members(pieces), name, declaration)
            .into_iter()
            .map(|piece| self.declaration_range(source, piece))
            .filter(|range| !range.is_empty())
            .filter(|range| !needs.contains(&range.start))
            .collect()
    }

    fn brace_body_after(&self, declaration: usize) -> Option<Range<usize>> {
        self.code[declaration..]
            .find('{')
            .and_then(|offset| self.brace_body(declaration + offset))
    }

    fn unless_defaulted(
        &self,
        pieces: Vec<Range<usize>>,
        name: &str,
        declaration: usize,
    ) -> Vec<Range<usize>> {
        if RequiredFields::rust_has_default(&self.code, name, declaration + 1) {
            return Vec::new();
        }
        pieces
    }

    fn members(&self, pieces: Vec<Range<usize>>) -> Vec<Range<usize>> {
        let least = pieces
            .iter()
            .filter(|piece| !piece.is_empty())
            .map(|piece| self.column(piece.start))
            .min()
            .unwrap_or(0);
        pieces
            .into_iter()
            .filter(|piece| self.column(piece.start) == least)
            .collect()
    }

    fn column(&self, at: usize) -> usize {
        at - self.code[..at].rfind('\n').map_or(0, |line| line + 1)
    }

    fn trimmed(&self, piece: Range<usize>) -> Range<usize> {
        let text = &self.code[piece.clone()];
        let start = piece.start + (text.len() - text.trim_start().len());
        let end = piece.start + text.trim_end().len();
        start.min(end)..end
    }

    fn declaration_range(&self, source: &str, piece: Range<usize>) -> Range<usize> {
        let (start, _) = self.declaration_start(self.code.as_bytes(), source, piece.start);
        start.min(piece.end)..piece.end
    }

    fn pieces(&self, body: Range<usize>) -> Vec<Range<usize>> {
        let depths = self.depths(body.clone());
        let cuts = body
            .clone()
            .filter(|at| self.is_cut(*at, depths[*at - body.start]))
            .collect::<Vec<_>>();
        Self::between(body, &cuts)
    }

    fn depths(&self, body: Range<usize>) -> Vec<i64> {
        let mut depth = 0_i64;
        body.map(|at| {
            depth += self.depth_change(self.code.as_bytes()[at]);
            depth
        })
        .collect()
    }

    fn depth_change(&self, byte: u8) -> i64 {
        let angles = i64::from(self.language == "rs");
        match byte {
            b'{' | b'(' | b'[' => 1,
            b'}' | b')' | b']' => -1,
            b'<' => angles,
            b'>' => -angles,
            _ => 0,
        }
    }

    fn is_cut(&self, at: usize, depth: i64) -> bool {
        depth == 0 && self.separators().contains(&self.code.as_bytes()[at])
    }

    fn separators(&self) -> &'static [u8] {
        Self::SEPARATORS
            .iter()
            .find(|(language, _)| *language == self.language)
            .map_or(Self::DEFAULT_SEPARATORS, |(_, separators)| *separators)
    }

    fn between(body: Range<usize>, cuts: &[usize]) -> Vec<Range<usize>> {
        let mut pieces = Vec::new();
        let mut start = body.start;
        for cut in cuts {
            pieces.push(start..*cut);
            start = cut + 1;
        }
        pieces.push(start..body.end);
        pieces
    }

    fn neighbour_problem(&self, index: usize, found: &Found, target: Target) -> Option<String> {
        let rule = found.rule;
        let language = self.language.as_str();
        let neighbours: Vec<(usize, &Found)> = self
            .found
            .iter()
            .enumerate()
            .filter(|(other_index, other)| {
                *other_index != index
                    && other.declaration == found.declaration
                    && other.target == Some(target)
            })
            .collect();
        for (other_index, other) in &neighbours {
            if other.rule.name == rule.name && *other_index < index {
                return Some(Messages::twice(rule, language, target));
            }
            if other.rule.name != rule.name && rule.clashes.contains(&other.rule.name) {
                return Some(Messages::clash(rule, other.rule, language, target));
            }
        }
        let has_partner = neighbours
            .iter()
            .any(|(_, other)| rule.with.contains(&other.rule.name));
        if !rule.with.is_empty() && !has_partner {
            return Some(Messages::without_partner(rule, language, target));
        }
        let repeated = rule.one_per_method
            && self.found[..index].iter().any(|other| {
                other.rule.name == rule.name
                    && other.method.is_some()
                    && other.method == found.method
            });
        if repeated {
            return Some(Messages::once_per_method(rule, language));
        }
        None
    }

    fn class_problem(found: &Found, language: &str, class_names: &[&str]) -> Option<String> {
        let rule = found.rule;
        if rule.class_needs.is_empty()
            || class_names
                .iter()
                .any(|name| rule.class_needs.contains(name))
        {
            return None;
        }
        Some(Messages::outside_class(rule, language))
    }
}

pub(crate) struct Messages;

impl Messages {
    const LUA_LATER: &'static str = " Lua gets it in a later step (ADR 0085).";
    const NOTHING_LATER: &'static str = "";
    pub(crate) fn spelled(language: &str, name: &str) -> String {
        match language {
            "py" => format!("@{}", Self::snake_case(name)),
            "rs" | "php" | "rb" => format!("#[{name}]"),
            "cs" => format!("[{name}]"),
            "go" => format!("// @{name}"),
            "lua" => format!("-- @{name}"),
            _ => format!("@{name}"),
        }
    }

    pub(crate) fn snake_case(name: &str) -> String {
        let characters: Vec<char> = name.chars().collect();
        let mut result = String::new();
        for (index, character) in characters.iter().enumerate() {
            if index > 0 && character.is_ascii_uppercase() {
                let previous = characters[index - 1];
                let next_is_lower = characters
                    .get(index + 1)
                    .is_some_and(char::is_ascii_lowercase);
                let starts_word = previous.is_ascii_lowercase()
                    || previous.is_ascii_digit()
                    || (previous.is_ascii_uppercase() && next_is_lower);
                if starts_word {
                    result.push('_');
                }
            }
            result.push(character.to_ascii_lowercase());
        }
        result
    }

    pub(crate) fn language_name(language: &str) -> &'static str {
        match language {
            "js" => "JavaScript",
            "ts" => "TypeScript",
            "py" => "Python",
            "php" => "PHP",
            "java" => "Java",
            "groovy" => "Groovy",
            "kt" => "Kotlin",
            "cs" => "C#",
            "rs" => "Rust",
            "dart" => "Dart",
            "go" => "Go",
            "swift" => "Swift",
            "rb" => "Ruby",
            "lua" => "Lua",
            _ => "this language",
        }
    }

    fn spelled_list(language: &str, names: &[&str]) -> String {
        let spelled: Vec<String> = names
            .iter()
            .map(|name| Self::spelled(language, name))
            .collect();
        spelled.join(" or ")
    }

    fn language_list(languages: &[&str]) -> String {
        let names: Vec<&str> = languages
            .iter()
            .map(|language| Self::language_name(language))
            .collect();
        names.join(", ")
    }

    pub(crate) fn unavailable(rule: &Rule, language: &str, realm: Realm) -> String {
        let spelled = Self::spelled(language, rule.name);
        let places = rule.languages(realm);
        let mut message = if places.is_empty() {
            format!(
                "{spelled} is not used in {} code. {}",
                realm.word(),
                rule.explained()
            )
        } else {
            format!(
                "{spelled} is not available in {} {} code; it works in {}.{} {}",
                Self::language_name(language),
                realm.word(),
                Self::language_list(places),
                Self::later(language),
                rule.explained()
            )
        };
        if !rule.advice.is_empty() {
            message = format!("{message} {}", rule.advice);
        }
        message
    }

    fn later(language: &str) -> &'static str {
        [Self::NOTHING_LATER, Self::LUA_LATER][usize::from(language == "lua")]
    }

    fn retired(rule: &Rule, language: &str) -> String {
        let spelled = Self::spelled(language, rule.name);
        if rule.replaced_by.is_empty() {
            return format!("{spelled} is not an annotation: {}", rule.advice);
        }
        format!(
            "{spelled} is retired: write {} instead. {}",
            Self::spelled(language, rule.replaced_by),
            rule.advice
        )
    }

    fn wrong_target(rule: &Rule, language: &str, target: Target) -> String {
        format!(
            "{} is on a {}, but it belongs on {}. {}",
            Self::spelled(language, rule.name),
            target.word(),
            rule.allowed_targets(),
            rule.explained()
        )
    }

    pub(crate) fn python_annotated(rule: &Rule, target: Target) -> String {
        let word = target.word();
        format!(
            "Python {word} markers go inside the {word}'s type, so the file stays valid Python: write `name: Annotated[Type, {}]` and import it with `from typing import Annotated`. {}",
            Self::snake_case(rule.name),
            rule.explained()
        )
    }

    pub(crate) fn constructor(rule: &Rule, language: &str) -> String {
        let written = match language {
            "py" => "__init__",
            "php" => "__construct",
            "rb" => "initialize",
            "swift" => "init",
            "rs" => "fn new",
            _ => "the constructor",
        };
        format!(
            "{} builds this class for you; remove {written} and declare the fields instead. {}",
            Self::spelled(language, rule.name),
            rule.explained()
        )
    }

    fn no_arguments(rule: &Rule, language: &str) -> String {
        if !rule.argument_advice.is_empty() {
            return format!(
                "{}: {}. Write {} alone. {}",
                Self::spelled(language, rule.name),
                rule.argument_advice,
                Self::spelled(language, rule.name),
                rule.explained()
            );
        }
        format!(
            "{} accepts no arguments: write {} alone. {}",
            rule.name,
            Self::spelled(language, rule.name),
            rule.explained()
        )
    }

    fn needs_arguments(rule: &Rule, language: &str) -> String {
        format!(
            "{} needs its arguments in parentheses. {}",
            Self::spelled(language, rule.name),
            rule.explained()
        )
    }

    fn not_static(rule: &Rule, language: &str, target: Target) -> String {
        format!(
            "{} is on a static {}, but it needs an instance {}. {}",
            Self::spelled(language, rule.name),
            target.word(),
            target.word(),
            rule.explained()
        )
    }

    fn twice(rule: &Rule, language: &str, target: Target) -> String {
        format!(
            "{} is written twice on one {}. Write it once. {}",
            Self::spelled(language, rule.name),
            target.word(),
            rule.explained()
        )
    }

    fn clash(rule: &Rule, other: &Rule, language: &str, target: Target) -> String {
        format!(
            "{} and {} cannot be on the same {}. {} {}",
            Self::spelled(language, rule.name),
            Self::spelled(language, other.name),
            target.word(),
            rule.explained(),
            other.explained()
        )
    }

    fn without_partner(rule: &Rule, language: &str, target: Target) -> String {
        format!(
            "{} needs {} on the same {}. {}",
            Self::spelled(language, rule.name),
            Self::spelled_list(language, rule.with),
            target.word(),
            rule.explained()
        )
    }

    fn once_per_method(rule: &Rule, language: &str) -> String {
        format!(
            "{} is on two parameters of one method; a method has at most one. {}",
            Self::spelled(language, rule.name),
            rule.explained()
        )
    }

    fn outside_class(rule: &Rule, language: &str) -> String {
        format!(
            "{} belongs in a class marked {}. {}",
            Self::spelled(language, rule.name),
            Self::spelled_list(language, rule.class_needs),
            rule.explained()
        )
    }
}
