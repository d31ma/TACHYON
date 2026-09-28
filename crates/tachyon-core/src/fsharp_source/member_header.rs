use super::{FsharpBody, FsharpParameters, FsharpType, FsharpValue, MemberWords};

pub(crate) struct MemberHeader {
    pub(crate) text: String,
    pub(crate) closer: String,
}

impl MemberHeader {
    const MODIFIERS: [&'static str; 13] = [
        "static", "member", "override", "default", "abstract", "private", "internal", "public",
        "inline", "val", "let", "mutable", "rec",
    ];
    const BINDING_ACCESS: &'static str = "private ";
    const CSHARP_MODIFIERS: [(&'static str, &'static str); 5] = [
        ("private", "private "),
        ("internal", "internal "),
        ("static", "static "),
        ("override", "override "),
        ("default", "virtual "),
    ];
    const MISSING_TYPE: &'static str = "object";
    const PROPERTY_ACCESSORS: &'static str = " with ";

    pub(crate) fn member(trimmed: &str, body_code: &[bool]) -> Self {
        let words = Self::words(trimmed);
        let modifiers = Self::csharp_modifiers(&words.modifiers);
        let rest = Self::without_self(words.rest, &words.modifiers);
        Self::shape(
            &modifiers,
            rest,
            &words.modifiers,
            Self::body_mask(trimmed, rest, body_code),
        )
    }

    pub(crate) fn value(trimmed: &str) -> Self {
        let words = Self::words(trimmed);
        let modifiers = Self::csharp_modifiers(&words.modifiers);
        let (declaration, initial) = words.rest.split_once('=').unwrap_or((words.rest, ""));
        let (name, kind) = declaration.split_once(':').unwrap_or((declaration, ""));
        let initial = initial
            .split(Self::PROPERTY_ACCESSORS)
            .next()
            .unwrap_or("")
            .trim();
        let kind = FsharpValue::type_or_guess(kind, initial);
        let accessors = Self::auto_accessors(words.rest);
        Self {
            text: format!("{modifiers}{kind} {} {accessors} = {initial}", name.trim()),
            closer: String::from(";"),
        }
    }

    pub(crate) fn binding(trimmed: &str, code: &[bool]) -> Self {
        let words = Self::words(trimmed);
        let static_word = Self::static_word(&words.modifiers);
        let modifiers = format!("{}{static_word}", Self::BINDING_ACCESS);
        let code = Self::body_mask(trimmed, words.rest, code);
        Self::binding_shape(&modifiers, words.rest, code)
    }

    fn static_word(modifiers: &[&str]) -> &'static str {
        if modifiers.contains(&"static") {
            return "static ";
        }
        ""
    }

    fn binding_shape(modifiers: &str, rest: &str, code: &[bool]) -> Self {
        let after_name = rest[Self::name_end(rest)..].trim_start();
        if after_name.starts_with('(') || after_name.starts_with(char::is_alphabetic) {
            Self::method(modifiers, rest, code)
        } else {
            Self::field(modifiers, rest, code)
        }
    }

    fn field(modifiers: &str, rest: &str, code: &[bool]) -> Self {
        let end = Self::name_end(rest);
        let name = &rest[..end];
        let (declaration, initial) = rest[end..].split_once('=').unwrap_or((&rest[end..], ""));
        let kind = FsharpValue::type_or_guess(declaration.trim().trim_start_matches(':'), initial);
        let initial_start = rest.len() - initial.len();
        let initial = FsharpBody::csharp(initial, code.get(initial_start..).unwrap_or(&[]));
        Self {
            text: format!("{modifiers}{kind} {name} ={initial}"),
            closer: String::from(";"),
        }
    }

    pub(crate) fn constructor(trimmed: &str, type_name: &str) -> Self {
        let after = trimmed.trim_start_matches("new").trim_start();
        let (parameters, rest) = FsharpParameters::read(after);
        let body = rest.trim_start_matches('=').trim();
        Self {
            text: format!("public {type_name}({parameters}) {{ {body}"),
            closer: String::from(" }"),
        }
    }

    fn words(trimmed: &str) -> MemberWords<'_> {
        let mut modifiers = Vec::new();
        let mut rest = trimmed;
        while let Some((word, next)) = Self::next_modifier(rest) {
            modifiers.push(word);
            rest = next;
        }
        MemberWords { modifiers, rest }
    }

    fn next_modifier(text: &str) -> Option<(&str, &str)> {
        let (word, next) = text.split_once(' ')?;
        Self::MODIFIERS
            .contains(&word)
            .then_some((word, next.trim_start()))
    }

    fn csharp_modifiers(modifiers: &[&str]) -> String {
        let access = Self::access(modifiers);
        let mut written = String::from(access);
        for (fsharp, csharp) in Self::CSHARP_MODIFIERS {
            Self::add_modifier(&mut written, modifiers, fsharp, csharp);
        }
        written
    }

    fn access(modifiers: &[&str]) -> &'static str {
        if modifiers.contains(&"private") || modifiers.contains(&"internal") {
            return "";
        }
        "public "
    }

    fn add_modifier(written: &mut String, modifiers: &[&str], fsharp: &str, csharp: &str) {
        if modifiers.contains(&fsharp) {
            written.push_str(csharp);
        }
    }

    fn without_self<'a>(rest: &'a str, modifiers: &[&str]) -> &'a str {
        let has_self = !modifiers.contains(&"static") && !modifiers.contains(&"abstract");
        Self::after_self_name(rest)
            .filter(|_| has_self)
            .unwrap_or(rest)
    }

    fn after_self_name(rest: &str) -> Option<&str> {
        let (self_name, member) = rest.split_once('.')?;
        let plain = self_name.chars().all(Self::is_name_character);
        plain.then_some(member)
    }

    fn body_mask<'a>(trimmed: &str, rest: &str, code: &'a [bool]) -> &'a [bool] {
        let skipped = trimmed.len() - rest.len();
        code.get(skipped..).unwrap_or(&[])
    }

    fn shape(modifiers: &str, rest: &str, words: &[&str], code: &[bool]) -> Self {
        if words.contains(&"abstract") {
            Self::abstract_member(modifiers, rest)
        } else if rest.contains(Self::PROPERTY_ACCESSORS) {
            Self::accessor_property(modifiers, rest)
        } else if rest[Self::name_end(rest)..].trim_start().starts_with('(') {
            Self::method(modifiers, rest, code)
        } else {
            Self::property(modifiers, rest, code)
        }
    }

    fn name_end(rest: &str) -> usize {
        rest.find(|character: char| !Self::is_name_character(character))
            .unwrap_or(rest.len())
    }

    fn is_name_character(character: char) -> bool {
        character.is_alphanumeric() || character == '_'
    }

    fn method(modifiers: &str, rest: &str, code: &[bool]) -> Self {
        let end = Self::name_end(rest);
        let name = &rest[..end];
        let (parameters, after) = FsharpParameters::read(&rest[end..]);
        let (kind, body) = Self::typed_body(after, rest, code);
        Self {
            text: format!("{modifiers}{kind} {name}({parameters}) {{ {body}"),
            closer: String::from(" }"),
        }
    }

    fn property(modifiers: &str, rest: &str, code: &[bool]) -> Self {
        let end = Self::name_end(rest);
        let name = &rest[..end];
        let (kind, body) = Self::typed_body(&rest[end..], rest, code);
        Self {
            text: format!("{modifiers}{kind} {name} {{ get {{ {body}"),
            closer: String::from(" } }"),
        }
    }

    fn accessor_property(modifiers: &str, rest: &str) -> Self {
        let end = Self::name_end(rest);
        let name = &rest[..end];
        let declaration = rest[end..]
            .split(Self::PROPERTY_ACCESSORS)
            .next()
            .unwrap_or("");
        let kind = Self::declared_type(declaration);
        Self {
            text: format!("{modifiers}{kind} {name} {{ get {{"),
            closer: String::from(" } }"),
        }
    }

    fn abstract_member(modifiers: &str, rest: &str) -> Self {
        let (name, signature) = rest.split_once(':').unwrap_or((rest, Self::MISSING_TYPE));
        let parts = FsharpType::split_top_level(signature, "->");
        let result = FsharpType::csharp(parts.last().copied().unwrap_or(Self::MISSING_TYPE));
        let parameters = Self::abstract_parameters(&parts[..parts.len() - 1]);
        Self {
            text: format!("{modifiers}abstract {result} {}({parameters})", name.trim()),
            closer: String::from(";"),
        }
    }

    fn abstract_parameters(parts: &[&str]) -> String {
        let mut parameters = Vec::new();
        for (index, part) in parts.iter().enumerate() {
            parameters.push(format!("{} value{index}", FsharpType::csharp(part)));
        }
        parameters.join(", ")
    }

    fn typed_body(after: &str, whole: &str, code: &[bool]) -> (String, String) {
        let (declaration, body) = after.split_once('=').unwrap_or((after, ""));
        let body_start = whole.len() - body.len();
        let body = FsharpBody::csharp(body, code.get(body_start..).unwrap_or(&[]));
        (Self::declared_type(declaration), body)
    }

    fn declared_type(declaration: &str) -> String {
        let kind = declaration.trim().trim_start_matches(':').trim();
        let kind = kind.split(Self::PROPERTY_ACCESSORS).next().unwrap_or(kind);
        let known = (!kind.is_empty()).then(|| FsharpType::csharp(kind));
        known.unwrap_or_else(|| String::from(Self::MISSING_TYPE))
    }

    fn auto_accessors(rest: &str) -> &'static str {
        if rest.contains("set") {
            return "{ get; set; }";
        }
        "{ get; }"
    }
}

#[cfg(test)]
mod tests {
    use super::MemberHeader;

    fn member(line: &str) -> (String, String) {
        let header = MemberHeader::member(line, &vec![true; line.len()]);
        (header.text, header.closer)
    }

    #[test]
    fn methods_become_public_csharp_methods() {
        assert_eq!(
            member("member this.GetOrder(id: int) : Order = store.Find(id)"),
            (
                String::from("public Order GetOrder(int id) {  store.Find(id)"),
                String::from(" }")
            )
        );
        assert_eq!(
            member("static member GET() : obj = box 1").0,
            "public static object GET() {  box 1"
        );
        assert_eq!(
            member("member private _.Check(value: float) = value > 0.0").0,
            "private object Check(double value) {  value > 0.0"
        );
    }

    #[test]
    fn properties_and_abstract_members() {
        assert_eq!(
            member("member this.Runtime : string = \"F#\"").0,
            "public string Runtime { get {  \"F#\""
        );
        assert_eq!(
            member("abstract member Find : int -> Order").0,
            "public abstract Order Find(int value0)"
        );
        assert_eq!(
            member("member this.Name with get () = name").0,
            "public object Name { get {"
        );
    }

    #[test]
    fn let_bindings_become_private_fields_and_methods() {
        let line = "let mutable count = 0";
        let field = MemberHeader::binding(line, &vec![true; line.len()]);
        assert_eq!(
            (field.text.as_str(), field.closer.as_str()),
            ("private int count = 0", ";")
        );
        let line = "let label (value: int) : string = string value";
        let method = MemberHeader::binding(line, &vec![true; line.len()]);
        assert_eq!(
            method.text,
            "private string label(int value) {  string value"
        );
    }

    #[test]
    fn auto_properties_and_constructors() {
        let value = MemberHeader::value("member val Count = 6 with get, set");
        assert_eq!(value.text, "public int Count { get; set; } = 6");
        let value = MemberHeader::value("member val Label : string = \"x\"");
        assert_eq!(value.text, "public string Label { get; } = \"x\"");
        let built = MemberHeader::constructor("new(count: int) = { count = count }", "Tally");
        assert_eq!(built.text, "public Tally(int count) { { count = count }");
    }
}
