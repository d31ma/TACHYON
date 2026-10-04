use std::ops::Range;

pub(crate) struct LexicalMask {
    pub(crate) code: Vec<bool>,
    pub(crate) error: Option<&'static str>,
    pub(crate) marker_problem: Option<(usize, &'static str)>,
}

impl LexicalMask {
    fn complete(code: Vec<bool>) -> Self {
        Self {
            code,
            error: None,
            marker_problem: None,
        }
    }

    fn malformed(mut code: Vec<bool>, start: usize, message: &'static str) -> Self {
        code[start..].fill(false);
        Self {
            code,
            error: Some(message),
            marker_problem: None,
        }
    }
}

enum Region {
    Code,
    Hidden { end: usize },
    Unterminated(&'static str),
}

impl Region {
    fn from_end(end: Option<usize>, message: &'static str) -> Self {
        match end {
            Some(end) => Self::Hidden { end },
            None => Self::Unterminated(message),
        }
    }
}

pub(crate) fn code_mask(extension: &str, source: &str) -> LexicalMask {
    if extension == "rb" {
        return crate::ruby_source::RubyLexer::code_mask(source);
    }
    if extension == crate::lua_source::LuaSource::EXTENSION {
        return crate::lua_source::LuaLexer::code_mask(source);
    }
    if extension == crate::fsharp_source::FsharpSource::EXTENSION {
        return crate::fsharp_source::FsharpLexer::code_mask(source);
    }
    let length = source.len();
    let mut code = vec![true; length];
    let mut at = 0;
    let mut hidden_tail: Option<Range<usize>> = None;
    let mut marker_problem = None;
    while at < length {
        if let Some(tail) = hidden_tail.take_if(|tail| at >= tail.start) {
            if at < tail.end {
                code[at..tail.end].fill(false);
                at = tail.end;
            }
            continue;
        }
        if extension == "go"
            && let Some(line) = GoMarkerLine::read(source, at)
        {
            marker_problem = marker_problem.or(line.problem);
            if line.exposed {
                code[at..at + 3].fill(false);
                hidden_tail = Some(line.exposed_end..line.line_end);
                at += 3;
                continue;
            }
        }
        match region_at(extension, source, at) {
            Region::Code => at += 1,
            Region::Hidden { end } => {
                code[at..end].fill(false);
                at = end;
            }
            Region::Unterminated(message) => {
                return LexicalMask::malformed(code, at, message);
            }
        }
    }
    LexicalMask {
        marker_problem,
        ..LexicalMask::complete(code)
    }
}

pub(crate) fn go_annotation_comment(source: &str, at: usize) -> bool {
    GoMarkerLine::read(source, at).is_some_and(|line| line.exposed)
}

struct GoMarkerLine {
    exposed: bool,
    exposed_end: usize,
    line_end: usize,
    problem: Option<(usize, &'static str)>,
}

impl GoMarkerLine {
    const SPACING: &'static str =
        "Write Go annotations as `// @Name`, with exactly one space after `//`";
    const UNCLOSED: &'static str =
        "Go annotation arguments must open and close on the annotation's own line";
    const TRAILING: &'static str =
        "A Go annotation line holds only the annotation; move the note to its own comment line";

    fn read(source: &str, at: usize) -> Option<Self> {
        let rest = source.get(at..)?.strip_prefix("//")?;
        let line_start = source[..at].rfind('\n').map_or(0, |index| index + 1);
        let before = source[line_start..at].trim_start_matches('\u{feff}');
        if !before.trim().is_empty() {
            return None;
        }
        let marker = rest.trim_start_matches([' ', '\t']);
        let spacing = &rest[..rest.len() - marker.len()];
        let marker = marker.strip_prefix('@')?;
        if !crate::go_source::GoMarkerName::is_marker(marker) {
            return None;
        }
        let at_sign = at + 2 + spacing.len();
        let line_end = source[at..]
            .find('\n')
            .map_or(source.len(), |index| at + index);
        if spacing != " " {
            return Some(Self {
                exposed: false,
                exposed_end: line_end,
                line_end,
                problem: Some((at_sign, Self::SPACING)),
            });
        }
        let name_end = at_sign + 1 + crate::go_source::GoMarkerName::read(marker).len();
        let mut line = Self {
            exposed: true,
            exposed_end: name_end,
            line_end,
            problem: None,
        };
        let after_name = &source[name_end..line_end];
        if after_name.starts_with('(') {
            match Self::group_length(after_name) {
                Some(length) => line.exposed_end = name_end + length,
                None => line.problem = Some((name_end, Self::UNCLOSED)),
            }
        }
        let tail = &source[line.exposed_end..line_end];
        if tail.trim().is_empty() {
            line.exposed_end = line_end;
        } else if line.problem.is_none() {
            let text_start = line.exposed_end + tail.len() - tail.trim_start().len();
            line.problem = Some((text_start, Self::TRAILING));
        }
        Some(line)
    }

    fn group_length(text: &str) -> Option<usize> {
        let bytes = text.as_bytes();
        let mut depth = 0_usize;
        let mut quote = None;
        let mut escaped = false;
        for (at, byte) in bytes.iter().enumerate() {
            if let Some(open) = quote {
                if escaped {
                    escaped = false;
                } else if *byte == b'\\' && open != b'`' {
                    escaped = true;
                } else if *byte == open {
                    quote = None;
                }
                continue;
            }
            match byte {
                b'"' | b'\'' | b'`' => quote = Some(*byte),
                b'(' => depth += 1,
                b')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(at + 1);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

fn region_at(extension: &str, source: &str, start: usize) -> Region {
    if let Some(region) = comment_region(extension, source.as_bytes(), start) {
        return region;
    }
    if let Some(region) = delimited_string_region(extension, source, start) {
        return region;
    }
    if let Some(region) = unescaped_string_region(extension, source, start) {
        return region;
    }
    let byte = source.as_bytes()[start];
    if byte == b'"' || byte == b'\'' {
        return quoted_string(extension, source.as_bytes(), start);
    }
    Region::Code
}

fn comment_region(extension: &str, bytes: &[u8], start: usize) -> Option<Region> {
    let byte = bytes[start];
    let next = bytes.get(start + 1).copied();
    if is_line_comment_start(extension, byte, next) {
        let end = line_end(bytes, start);
        return Some(Region::Hidden { end });
    }
    if byte == b'/' && next == Some(b'*') {
        let end = block_comment_end(extension, bytes, start);
        return Some(Region::from_end(end, "Unterminated block comment"));
    }
    None
}

fn delimited_string_region(extension: &str, source: &str, start: usize) -> Option<Region> {
    let bytes = source.as_bytes();
    let byte = bytes[start];
    let first_three = bytes.get(start..start + 3);
    if extension == "dart" && matches!(byte, b'\'' | b'"') {
        let end = dart_string_end(bytes, start);
        return Some(Region::from_end(end, "Unterminated Dart string"));
    }
    if extension == "groovy" && matches!(byte, b'\'' | b'"') {
        let end = dart_string_end(bytes, start);
        return Some(Region::from_end(end, "Unterminated Groovy string"));
    }
    let has_triple_strings = matches!(extension, "java" | "kt" | "cs" | "py");
    if has_triple_strings && first_three == Some(&b"\"\"\""[..]) {
        let end = text_end(source, start + 3, "\"\"\"");
        return Some(Region::from_end(end, "Unterminated multiline string"));
    }
    if extension == "py" && first_three == Some(&b"'''"[..]) {
        let end = text_end(source, start + 3, "'''");
        return Some(Region::from_end(
            end,
            "Unterminated Python multiline string",
        ));
    }
    if matches!(extension, "js" | "ts") && byte == b'`' {
        let end = InterpolatedString::JAVASCRIPT.end(bytes, start);
        return Some(Region::from_end(
            end,
            "Unterminated JavaScript template string",
        ));
    }
    if extension == "cs" && byte == b'$' && is_csharp_interpolated_start(bytes, start) {
        let end = InterpolatedString::CSHARP.end(bytes, start + 1);
        return Some(Region::from_end(end, "Unterminated C# interpolated string"));
    }
    None
}

fn is_csharp_interpolated_start(bytes: &[u8], start: usize) -> bool {
    bytes.get(start + 1) == Some(&b'"') && bytes.get(start + 1..start + 4) != Some(&b"\"\"\""[..])
}

enum StringPart {
    Text,
    Code { open_braces: usize },
}

struct InterpolatedString {
    quote: u8,
    code_opener: &'static [u8],
    nested_string_opener: &'static [u8],
    doubled_brace_is_text: bool,
}

impl InterpolatedString {
    const JAVASCRIPT: Self = Self {
        quote: b'`',
        code_opener: b"${",
        nested_string_opener: b"`",
        doubled_brace_is_text: false,
    };
    const CSHARP: Self = Self {
        quote: b'"',
        code_opener: b"{",
        nested_string_opener: b"$\"",
        doubled_brace_is_text: true,
    };

    fn end(&self, bytes: &[u8], quote_at: usize) -> Option<usize> {
        let mut parts = vec![StringPart::Text];
        let mut at = quote_at + 1;
        while let Some(part) = parts.last_mut() {
            let byte = *bytes.get(at)?;
            let StringPart::Code { open_braces } = part else {
                at = self.after_text_byte(bytes, at, &mut parts);
                continue;
            };
            if byte == b'{' {
                *open_braces += 1;
                at += 1;
            } else if byte == b'}' && *open_braces == 0 {
                parts.pop();
                at += 1;
            } else if byte == b'}' {
                *open_braces -= 1;
                at += 1;
            } else {
                at = self.after_code_byte(bytes, at, &mut parts)?;
            }
        }
        Some(at)
    }

    fn after_text_byte(&self, bytes: &[u8], at: usize, parts: &mut Vec<StringPart>) -> usize {
        if bytes[at] == b'\\' {
            return at + 2;
        }
        if bytes[at] == self.quote {
            parts.pop();
            return at + 1;
        }
        if self.doubled_brace_is_text && bytes[at..].starts_with(b"{{") {
            return at + 2;
        }
        if bytes[at..].starts_with(self.code_opener) {
            parts.push(StringPart::Code { open_braces: 0 });
            return at + self.code_opener.len();
        }
        at + 1
    }

    fn after_code_byte(
        &self,
        bytes: &[u8],
        at: usize,
        parts: &mut Vec<StringPart>,
    ) -> Option<usize> {
        if bytes[at..].starts_with(self.nested_string_opener) {
            parts.push(StringPart::Text);
            return Some(at + self.nested_string_opener.len());
        }
        if matches!(bytes[at], b'\'' | b'"') {
            return escaped_string_end(bytes, at);
        }
        Some(at + 1)
    }
}

fn unescaped_string_region(extension: &str, source: &str, start: usize) -> Option<Region> {
    let bytes = source.as_bytes();
    let byte = bytes[start];
    if extension == "rs" && byte == b'r' {
        return Some(rust_raw_string(source, start));
    }
    if extension == "swift" && byte == b'#' {
        return Some(swift_raw_string(source, start));
    }
    if extension == "dart" && is_dart_raw_string_start(bytes, start) {
        let end = dart_raw_string_end(source, start);
        return Some(Region::from_end(end, "Unterminated Dart raw string"));
    }
    if extension == crate::tac_cpp::CppSource::EXTENSION
        && crate::tac_cpp::CppRawText::is_raw_string_start(bytes, start)
    {
        let end = crate::tac_cpp::CppRawText::raw_string_end(source, start);
        return Some(Region::from_end(end, "Unterminated C++ raw string"));
    }
    if extension == "go" && byte == b'`' {
        let end = text_end(source, start + 1, "`");
        return Some(Region::from_end(end, "Unterminated Go raw string"));
    }
    if extension == "cs" && byte == b'@' && bytes.get(start + 1) == Some(&b'"') {
        let end = csharp_verbatim_end(bytes, start + 2);
        return Some(Region::from_end(end, "Unterminated C# verbatim string"));
    }
    if extension == "php" && bytes.get(start..start + 3) == Some(&b"<<<"[..]) {
        let region = match php_heredoc_end(source, start) {
            Ok(end) => Region::Hidden { end },
            Err(message) => Region::Unterminated(message),
        };
        return Some(region);
    }
    None
}

fn is_line_comment_start(extension: &str, byte: u8, next: Option<u8>) -> bool {
    if byte == b'/' && next == Some(b'/') {
        return true;
    }
    matches!(extension, "php" | "py") && byte == b'#' && next != Some(b'[')
}

fn line_end(bytes: &[u8], start: usize) -> usize {
    let mut at = start;
    while at < bytes.len() && bytes[at] != b'\n' {
        at += 1;
    }
    at
}

fn block_comment_end(extension: &str, bytes: &[u8], start: usize) -> Option<usize> {
    let allows_nesting = matches!(extension, "rs" | "dart");
    let mut at = start + 2;
    let mut depth = 1_usize;
    while at + 1 < bytes.len() {
        let opens = bytes[at] == b'/' && bytes[at + 1] == b'*';
        let closes = bytes[at] == b'*' && bytes[at + 1] == b'/';
        if allows_nesting && opens {
            depth += 1;
            at += 2;
        } else if closes {
            depth -= 1;
            at += 2;
            if depth == 0 {
                return Some(at);
            }
        } else {
            at += 1;
        }
    }
    None
}

fn text_end(source: &str, from: usize, closing: &str) -> Option<usize> {
    let offset = source[from..].find(closing)?;
    Some(from + offset + closing.len())
}

fn escaped_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let quote = bytes[start];
    let mut at = start + 1;
    while at < bytes.len() {
        if bytes[at] == b'\\' {
            at = (at + 2).min(bytes.len());
        } else if bytes[at] == quote {
            return Some(at + 1);
        } else {
            at += 1;
        }
    }
    None
}

fn quoted_string(extension: &str, bytes: &[u8], start: usize) -> Region {
    if extension == "rs" && is_rust_lifetime(bytes, start) {
        return Region::Code;
    }
    if extension == crate::tac_cpp::CppSource::EXTENSION
        && crate::tac_cpp::CppRawText::is_digit_separator(bytes, start)
    {
        return Region::Code;
    }
    let end = escaped_string_end(bytes, start);
    Region::from_end(end, "Unterminated string")
}

fn is_rust_lifetime(bytes: &[u8], start: usize) -> bool {
    if bytes[start] != b'\'' {
        return false;
    }
    let starts_name = bytes
        .get(start + 1)
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_');
    starts_name && bytes.get(start + 2) != Some(&b'\'')
}

fn rust_raw_string(source: &str, start: usize) -> Region {
    let bytes = source.as_bytes();
    let mut quote = start + 1;
    while bytes.get(quote) == Some(&b'#') {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'"') {
        return Region::Code;
    }
    let hashes = quote - start - 1;
    let closing = format!("\"{}", "#".repeat(hashes));
    let end = text_end(source, quote + 1, &closing);
    Region::from_end(end, "Unterminated Rust raw string")
}

fn swift_raw_string(source: &str, start: usize) -> Region {
    let bytes = source.as_bytes();
    let mut quote = start;
    while bytes.get(quote) == Some(&b'#') {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'"') {
        return Region::Code;
    }
    let closing = format!("\"{}", "#".repeat(quote - start));
    let end = text_end(source, quote + 1, &closing);
    Region::from_end(end, "Unterminated Swift raw string")
}

fn is_dart_raw_string_start(bytes: &[u8], start: usize) -> bool {
    if bytes[start] != b'r' || !matches!(bytes.get(start + 1), Some(b'\'' | b'"')) {
        return false;
    }
    let Some(before) = start.checked_sub(1) else {
        return true;
    };
    let previous = bytes[before];
    let continues_name = previous.is_ascii_alphanumeric() || previous == b'_' || previous == b'$';
    !continues_name
}

fn dart_raw_string_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let quote = bytes[start + 1];
    let triple = bytes.get(start + 1..start + 4) == Some(&[quote, quote, quote][..]);
    let quote_text = String::from(quote as char);
    if triple {
        return text_end(source, start + 4, &quote_text.repeat(3));
    }
    text_end(source, start + 2, &quote_text)
}

fn csharp_verbatim_end(bytes: &[u8], mut at: usize) -> Option<usize> {
    loop {
        let quote = bytes[at..].iter().position(|byte| *byte == b'"')?;
        at += quote + 1;
        if bytes.get(at) != Some(&b'"') {
            return Some(at);
        }
        at += 1;
    }
}

fn php_heredoc_end(source: &str, start: usize) -> Result<usize, &'static str> {
    let length = source.len();
    let line_end = source[start..]
        .find('\n')
        .map(|offset| start + offset)
        .ok_or("Malformed PHP heredoc")?;
    let label = source[start + 3..line_end].trim().trim_matches(['\'', '"']);
    if !is_php_heredoc_label(label) {
        return Err("Malformed PHP heredoc label");
    }
    let mut at = line_end + 1;
    while at <= length {
        let next_line_end = source[at..].find('\n').map_or(length, |offset| at + offset);
        let line = source[at..next_line_end].trim().trim_end_matches(';');
        if line == label {
            if next_line_end < length {
                return Ok(next_line_end + 1);
            }
            return Ok(next_line_end);
        }
        if next_line_end == length {
            break;
        }
        at = next_line_end + 1;
    }
    Err("Unterminated PHP heredoc")
}

fn is_php_heredoc_label(label: &str) -> bool {
    if label.is_empty() {
        return false;
    }
    label
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn dart_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let quote = bytes[start];
    let triple_quote = [quote, quote, quote];
    let triple = bytes.get(start..start + 3) == Some(&triple_quote[..]);
    let mut index = start + if triple { 3 } else { 1 };
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'$' if bytes.get(index + 1) == Some(&b'{') => {
                index = dart_interpolation_end(bytes, index + 2)?;
            }
            byte if byte == quote => {
                if !triple {
                    return Some(index + 1);
                }
                if bytes.get(index..index + 3) == Some(&triple_quote[..]) {
                    return Some(index + 3);
                }
                index += 1;
            }
            b'\n' if !triple => return None,
            _ => index += 1,
        }
    }
    None
}

fn dart_interpolation_end(bytes: &[u8], mut at: usize) -> Option<usize> {
    let mut depth = 1_usize;
    while at < bytes.len() {
        match bytes[at] {
            b'{' => {
                depth += 1;
                at += 1;
            }
            b'}' => {
                depth -= 1;
                at += 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            b'\'' | b'"' => at = dart_string_end(bytes, at)?,
            b'r' if matches!(bytes.get(at + 1), Some(b'\'' | b'"')) => {
                let quote = bytes[at + 1];
                let close = bytes[at + 2..].iter().position(|byte| *byte == quote)?;
                at += 2 + close + 1;
            }
            _ => at += 1,
        }
    }
    None
}

#[cfg(test)]
mod dart_tests {
    use super::code_mask;

    fn visible_code(extension: &str, source: &str) -> String {
        let mask = code_mask(extension, source);
        assert!(mask.error.is_none(), "{source}: {:?}", mask.error);
        source
            .char_indices()
            .map(|(at, value)| if mask.code[at] { value } else { ' ' })
            .collect()
    }

    #[test]
    fn a_dart_interpolation_may_hold_its_own_quotes() {
        let source = "log.add('wide:${value['status']}'); GET(); var t = \"a ${m[\"k\"] + '}'} b\"; POST();\n";
        let masked = visible_code("dart", source);
        assert!(
            !masked.contains("status") && !masked.contains("wide"),
            "{masked}"
        );
        assert!(
            masked.contains("GET()") && masked.contains("POST()"),
            "{masked}"
        );
    }

    #[test]
    fn dart_comments_and_strings_are_not_code() {
        let source = concat!(
            "/* outer /* inner */ still comment */ GET();\n",
            "var a = r'C:\\'; var b = '''x\nGET()''';\n",
            "var c = \"\"\"y\"\"\"; POST();\n",
        );
        let masked = visible_code("dart", source);
        assert!(
            !masked.contains("outer") && !masked.contains("still"),
            "{masked}"
        );
        assert_eq!(masked.matches("GET").count(), 1, "{masked}");
        assert!(masked.contains("POST"), "{masked}");
        assert!(!masked.contains("C:") && !masked.contains('y'), "{masked}");
    }

    #[test]
    fn groovy_strings_hide_quotes_braces_and_interpolations() {
        let source = concat!(
            "String a = 'it\\'s {'; GET();\n",
            "String b = \"total ${order.name(\"x}\")} }\"; PUT();\n",
            "String c = '''one\n{ two'''; String d = \"\"\"three\n}\"\"\"; POST();\n",
        );
        let masked = visible_code("groovy", source);
        assert!(!masked.contains('{') && !masked.contains('}'), "{masked}");
        assert!(
            !masked.contains("total") && !masked.contains("two"),
            "{masked}"
        );
        assert!(
            masked.contains("GET()") && masked.contains("PUT()") && masked.contains("POST()"),
            "{masked}"
        );
    }
}

#[cfg(test)]
mod interpolated_string_tests {
    use super::code_mask;

    fn visible_code(extension: &str, source: &str) -> String {
        let mask = code_mask(extension, source);
        assert!(mask.error.is_none(), "{source}: {:?}", mask.error);
        source
            .char_indices()
            .map(|(at, value)| if mask.code[at] { value } else { ' ' })
            .collect()
    }

    #[test]
    fn nested_javascript_templates_hide_their_braces() {
        let source = "a = `x ${ok ? `{` : `${`}`}`} y`; GET();";
        let masked = visible_code("js", source);
        assert!(!masked.contains('{') && !masked.contains('}'), "{masked}");
        assert!(masked.contains("GET()"), "{masked}");
    }

    #[test]
    fn csharp_interpolated_strings_hide_their_braces() {
        let source = "a = $\"{{x}} {(ok ? \"}\" : $\"{b}\")}\"; GET();";
        let masked = visible_code("cs", source);
        assert!(!masked.contains('{') && !masked.contains('}'), "{masked}");
        assert!(masked.contains("GET()"), "{masked}");
    }

    #[test]
    fn go_raw_strings_hide_quotes_braces_and_comment_marks() {
        let source = "a := `x \" } // y\n'z'`; GET()";
        let masked = visible_code("go", source);
        assert!(!masked.contains('}') && !masked.contains("//"), "{masked}");
        assert!(masked.contains("GET()"), "{masked}");
    }

    #[test]
    fn go_comment_markers_preserve_unicode_code_and_hide_literal_lookalikes() {
        let source = "type Café struct{}\n// @Service\nvar 日本 = `// @Wrong`; var text = \"λ\"\n/* // @Wrong */\n// café\n";
        let masked = visible_code("go", source);
        assert!(masked.contains("Café") && masked.contains("日本"));
        assert!(masked.contains("@Service"));
        assert!(!masked.contains("@Wrong") && !masked.contains("café"));
    }

    #[test]
    fn swift_raw_strings_end_at_their_own_hashes() {
        let source = "a = ##\"x \"# } \"## ; #if DEBUG\nGET()";
        let masked = visible_code("swift", source);
        assert!(!masked.contains('}'), "{masked}");
        assert!(
            masked.contains("#if DEBUG") && masked.contains("GET()"),
            "{masked}"
        );
    }
}
