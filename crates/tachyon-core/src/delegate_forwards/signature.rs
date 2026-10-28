use super::{MarkerStrip, MethodHeader, ParameterSplit, SourceLines};

pub(super) struct Signature {
    pub(super) parameters: String,
    pub(super) returns: String,
}

impl Signature {
    const TYPE_AFTER_COLON: [&'static str; 3] = ["kt", "fs", "ts"];
    const ARROW_RETURNS: [&'static str; 2] = ["rs", "swift"];
    const COLON_RETURNS: [&'static str; 2] = ["kt", "fs"];
    const KOTLIN_SPREAD: &'static str = "vararg";
    const DART_REQUIRED: &'static str = "required";
    const ARROW: &'static str = "->";
    const MODIFIERS: [&'static str; 22] = [
        "public",
        "private",
        "protected",
        "internal",
        "static",
        "final",
        "abstract",
        "synchronized",
        "override",
        "virtual",
        "sealed",
        "async",
        "new",
        "inline",
        "constexpr",
        "explicit",
        "default",
        "open",
        "extern",
        "unsafe",
        "partial",
        "native",
    ];

    pub(super) fn of(language: &str, name: &str, header: &MethodHeader) -> Self {
        let declaration = &header.declaration;
        let (before, after) = Self::around_parameters(declaration, name);
        let types = Self::parameter_types(language, &header.parameters);
        let qualifiers = Self::plain(["", after][usize::from(language == "hpp")]);
        Self {
            parameters: format!("{}|{qualifiers}", types.join(",")),
            returns: Self::plain(&Self::returns(language, before, after)),
        }
    }

    fn around_parameters<'a>(declaration: &'a str, name: &str) -> (&'a str, &'a str) {
        let name_at = declaration
            .match_indices(name)
            .map(|(offset, _)| offset)
            .find(|offset| Self::names_method(declaration, *offset, name.len()))
            .unwrap_or_default();
        let open = declaration[name_at..]
            .find('(')
            .map_or(declaration.len(), |offset| name_at + offset);
        let close = ParameterSplit::matching_close(declaration, open).unwrap_or(declaration.len());
        let after = declaration.get(close + 1..).unwrap_or_default();
        (&declaration[..name_at], after)
    }

    fn names_method(declaration: &str, offset: usize, length: usize) -> bool {
        let before = declaration[..offset].chars().next_back();
        let after = declaration[offset + length..].trim_start();
        let alone = before.is_none_or(|letter| !SourceLines::is_word_letter(letter));
        [alone, after.starts_with(['(', '<'])] == [true, true]
    }

    fn returns(language: &str, before: &str, after: &str) -> String {
        let arrow = after
            .split_once(Self::ARROW)
            .map_or("", |(_, written)| written);
        let colon = after.trim_start().strip_prefix(':').unwrap_or_default();
        let picked = [
            (language == "go").then_some(after),
            Self::ARROW_RETURNS.contains(&language).then_some(arrow),
            Self::COLON_RETURNS.contains(&language).then_some(colon),
        ];
        picked
            .into_iter()
            .flatten()
            .next()
            .map_or_else(|| Self::type_words(before), str::to_owned)
    }

    fn type_words(before: &str) -> String {
        before
            .split_whitespace()
            .filter(|word| !Self::MODIFIERS.contains(word))
            .collect()
    }

    fn parameter_types(language: &str, parameters: &[String]) -> Vec<String> {
        let plain: Vec<String> = parameters
            .iter()
            .map(|parameter| MarkerStrip::plain(language, parameter))
            .collect();
        match language {
            "go" => Self::go_types(&plain),
            _ => plain
                .iter()
                .filter_map(|parameter| Self::parameter_type(language, parameter))
                .collect(),
        }
    }

    fn parameter_type(language: &str, parameter: &str) -> Option<String> {
        let declared = parameter.split('=').next().unwrap_or(parameter).trim();
        let written = match language {
            "swift" => Some(Self::swift_type(declared)),
            "dart" => Some(Self::dart_type(parameter)),
            "rs" => declared.contains(':').then(|| Self::colon_type(declared)),
            typed if Self::TYPE_AFTER_COLON.contains(&typed) => Some(Self::colon_type(declared)),
            _ => Some(Self::without_name(declared).to_owned()),
        };
        written.map(|text| Self::plain(&text))
    }

    fn colon_type(declared: &str) -> String {
        let spread = declared.starts_with(Self::KOTLIN_SPREAD);
        let written = declared.split_once(':').map_or("", |(_, written)| written);
        format!(
            "{}{written}",
            ["", Self::KOTLIN_SPREAD][usize::from(spread)]
        )
    }

    fn swift_type(declared: &str) -> String {
        let (names, written) = declared.split_once(':').unwrap_or((declared, ""));
        let label = names.split_whitespace().next().unwrap_or_default();
        format!("{label}:{written}")
    }

    fn dart_type(parameter: &str) -> String {
        let grouped = parameter.starts_with(['{', '[']);
        let positional = parameter.split('=').next().unwrap_or(parameter).trim();
        let text = [Self::without_name(positional), parameter][usize::from(grouped)];
        text.trim_start_matches(Self::DART_REQUIRED).to_owned()
    }

    fn without_name(declared: &str) -> &str {
        let trimmed = declared.trim_end();
        let start = trimmed
            .rfind(|letter: char| !SourceLines::is_word_letter(letter))
            .map_or(0, |offset| offset + 1);
        &trimmed[..start]
    }

    fn go_types(parts: &[String]) -> Vec<String> {
        let mut carried = String::new();
        let mut types: Vec<String> = parts
            .iter()
            .rev()
            .map(|part| {
                let written = part.trim().split_once(char::is_whitespace);
                carried = written.map_or_else(|| carried.clone(), |(_, kind)| Self::plain(kind));
                carried.clone()
            })
            .collect();
        types.reverse();
        types
    }

    fn plain(text: &str) -> String {
        text.split_whitespace().collect()
    }
}
