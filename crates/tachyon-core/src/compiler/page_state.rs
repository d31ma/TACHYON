use crate::Failure;
use crate::failure::{diagnostic, source_span};

pub(super) struct PageState;

impl PageState {
    const OPEN_TAG: &'static str = "<script";
    const CLOSE_TAG: &'static str = "</script>";
    const MAX_TEMPLATE_BYTES: usize = 1_048_576;
    const DECLARATION_KEYWORDS: [&'static str; 3] = ["let ", "const ", "var "];

    pub(super) fn strip_scripts(
        source: &str,
        source_path: &str,
    ) -> Result<(String, String), Failure> {
        let mut template = source.as_bytes().to_vec();
        let mut state = String::new();
        Self::extract_blocks(source, source_path, &mut template, &mut state)?;
        let template = Self::utf8_template(template, source, source_path)?;
        Ok((template, state))
    }

    pub(super) fn mask_script_blocks(source: &str) -> String {
        let mut template = source.as_bytes().to_vec();
        Self::blank_script_blocks(source, &mut template);
        String::from_utf8_lossy(&template).into_owned()
    }

    pub(super) fn read_template_source(bytes: &[u8], source_path: &str) -> Result<String, Failure> {
        Self::check_template_size(bytes, source_path)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| Self::utf8_failure(error.valid_up_to(), source_path))
    }

    pub(super) fn unquoted(text: &str) -> Option<&str> {
        ['\'', '"']
            .iter()
            .find_map(|quote| text.strip_prefix(*quote)?.strip_suffix(*quote))
    }

    fn extract_blocks(
        source: &str,
        source_path: &str,
        template: &mut [u8],
        state: &mut String,
    ) -> Result<(), Failure> {
        let mut cursor = 0;
        while let Some((start, open_end, close)) = Self::next_block(source, cursor, source_path)? {
            state.push_str(&source[open_end..close]);
            state.push('\n');
            let end = close + Self::CLOSE_TAG.len();
            Self::blank_out(&mut template[start..end]);
            cursor = end;
        }
        Ok(())
    }

    fn next_block(
        source: &str,
        cursor: usize,
        source_path: &str,
    ) -> Result<Option<(usize, usize, usize)>, Failure> {
        Self::opening_tag(source, cursor)
            .map(|(start, open_end)| Self::closed_block(source, source_path, start, open_end))
            .transpose()
    }

    fn opening_tag(source: &str, cursor: usize) -> Option<(usize, usize)> {
        let start = cursor + source[cursor..].find(Self::OPEN_TAG)?;
        let open_end = start + source[start..].find('>')? + 1;
        Some((start, open_end))
    }

    fn closed_block(
        source: &str,
        source_path: &str,
        start: usize,
        open_end: usize,
    ) -> Result<(usize, usize, usize), Failure> {
        Self::check_opening(&source[start..open_end], source_path, start, open_end)?;
        let close_relative = source[open_end..]
            .find(Self::CLOSE_TAG)
            .ok_or_else(|| Self::unclosed_failure(source_path, start, open_end))?;
        let close = open_end + close_relative;
        Self::validate_state(&source[open_end..close], source_path, open_end)?;
        Ok((start, open_end, close))
    }

    fn utf8_template(
        template: Vec<u8>,
        source: &str,
        source_path: &str,
    ) -> Result<String, Failure> {
        String::from_utf8(template).map_err(|_| {
            Failure::one(diagnostic(
                1301,
                "Template source is not UTF-8.",
                None,
                source_span(source_path, 0, source.len()),
            ))
        })
    }

    fn check_opening(
        opening: &str,
        source_path: &str,
        start: usize,
        end: usize,
    ) -> Result<(), Failure> {
        if opening.trim() == "<script>" {
            return Ok(());
        }
        Err(Failure::one(diagnostic(
            1306,
            "A Tac page state script cannot declare HTML attributes.",
            Some(String::from(
                "Use a plain <script> block for state declarations and a colocated tac.js for behavior.",
            )),
            source_span(source_path, start, end),
        )))
    }

    fn unclosed_failure(source_path: &str, start: usize, end: usize) -> Failure {
        Failure::one(diagnostic(
            1301,
            "Tac page state script is not closed.",
            Some(String::from("Add </script>.")),
            source_span(source_path, start, end),
        ))
    }

    fn blank_out(bytes: &mut [u8]) {
        for byte in bytes {
            *byte = Self::blanked(*byte);
        }
    }

    fn blanked(byte: u8) -> u8 {
        if byte == b'\n' || byte == b'\r' {
            return byte;
        }
        b' '
    }

    fn blank_script_blocks(source: &str, template: &mut [u8]) {
        let mut cursor = 0;
        while let Some(relative) = source[cursor..].find(Self::OPEN_TAG) {
            let start = cursor + relative;
            let end = Self::script_end(source, start);
            Self::blank_out(&mut template[start..end]);
            cursor = end;
        }
    }

    fn script_end(source: &str, start: usize) -> usize {
        source[start..]
            .find(Self::CLOSE_TAG)
            .map_or(source.len(), |relative| {
                start + relative + Self::CLOSE_TAG.len()
            })
    }

    fn validate_state(source: &str, source_path: &str, offset: usize) -> Result<(), Failure> {
        let invalid = source
            .split([';', '\n'])
            .map(str::trim)
            .find(|statement| Self::is_invalid_statement(statement));
        invalid.map_or(Ok(()), |statement| {
            Err(Self::state_failure(source, statement, source_path, offset))
        })
    }

    fn is_invalid_statement(statement: &str) -> bool {
        !statement.is_empty() && !Self::is_literal_declaration(statement)
    }

    fn state_failure(source: &str, statement: &str, source_path: &str, offset: usize) -> Failure {
        let start = offset.saturating_add(source.find(statement).unwrap_or(0));
        let end = start.saturating_add(statement.len());
        Failure::one(diagnostic(
            1306,
            "A Tac page state script may contain only literal variable declarations.",
            Some(String::from(
                "Move behavior into the colocated tac.js module; keep only let, const, or var declarations with bounded JSON-like literal values here.",
            )),
            source_span(source_path, start, end),
        ))
    }

    fn is_literal_declaration(statement: &str) -> bool {
        Self::literal_declaration(statement).is_some()
    }

    fn literal_declaration(statement: &str) -> Option<serde_json::Value> {
        let declaration = Self::DECLARATION_KEYWORDS
            .iter()
            .find_map(|keyword| statement.strip_prefix(keyword))?;
        let (_, value) = declaration
            .split_once('=')
            .filter(|(name, _)| Self::valid_page_field(name.trim()))?;
        Self::parse_javascript_literal(value)
    }

    fn check_template_size(bytes: &[u8], source_path: &str) -> Result<(), Failure> {
        if bytes.len() > Self::MAX_TEMPLATE_BYTES {
            return Err(Failure::one(diagnostic(
                1301,
                "Template source exceeds the 1 MiB limit.",
                Some(String::from("Split the view into bounded Tac components.")),
                source_span(source_path, 0, bytes.len()),
            )));
        }
        Ok(())
    }

    fn utf8_failure(start: usize, source_path: &str) -> Failure {
        Failure::one(diagnostic(
            1301,
            "Template source must be valid UTF-8.",
            None,
            source_span(source_path, start, start.saturating_add(1)),
        ))
    }

    fn valid_page_field(value: &str) -> bool {
        let mut bytes = value.bytes();
        let first = bytes.next().is_some_and(Self::is_field_start);
        let rest = bytes.all(Self::is_field_byte);
        first && rest
    }

    fn is_field_start(byte: u8) -> bool {
        byte.is_ascii_alphabetic() || byte == b'_' || byte == b'$'
    }

    fn is_field_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
    }

    fn parse_javascript_literal(source: &str) -> Option<serde_json::Value> {
        let source = source.trim();
        if source.starts_with('[') && source.ends_with(']') {
            return Self::array_literal(source);
        }
        Self::scalar_literal(source)
    }

    fn array_literal(source: &str) -> Option<serde_json::Value> {
        let inner = &source[1..source.len() - 1];
        let items: Option<Vec<serde_json::Value>> = Self::array_items(inner)
            .into_iter()
            .map(Self::parse_javascript_literal)
            .collect();
        items.map(serde_json::Value::Array)
    }

    fn array_items(inner: &str) -> Vec<&str> {
        if inner.trim().is_empty() {
            return Vec::new();
        }
        inner.split(',').collect()
    }

    fn scalar_literal(source: &str) -> Option<serde_json::Value> {
        Self::unquoted(source)
            .map(|text| serde_json::Value::String(String::from(text)))
            .or_else(|| serde_json::from_str(source).ok())
    }
}
