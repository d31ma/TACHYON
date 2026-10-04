pub(crate) struct CppRawText;

impl CppRawText {
    const RAW_OPENING: &'static [u8] = b"R\"";

    pub(crate) fn is_raw_string_start(bytes: &[u8], start: usize) -> bool {
        let opens = bytes.get(start..start + 2) == Some(Self::RAW_OPENING);
        let previous = start.checked_sub(1).and_then(|index| bytes.get(index));
        let joined = previous
            .is_some_and(|byte| [byte.is_ascii_alphanumeric(), *byte == b'_'].contains(&true));
        [opens, joined] == [true, false]
    }

    pub(crate) fn raw_string_end(source: &str, start: usize) -> Option<usize> {
        let open = start + 2;
        let parenthesis = source.get(open..)?.find('(')?;
        let delimiter = &source[open..open + parenthesis];
        let closing = format!("){delimiter}\"");
        let body = open + parenthesis + 1;
        let offset = source.get(body..)?.find(&closing)?;
        Some(body + offset + closing.len())
    }

    pub(crate) fn is_digit_separator(bytes: &[u8], start: usize) -> bool {
        let previous = start.checked_sub(1).and_then(|index| bytes.get(index));
        let next = bytes.get(start + 1);
        let hex = [
            previous.is_some_and(u8::is_ascii_hexdigit),
            next.is_some_and(u8::is_ascii_hexdigit),
        ];
        hex == [true, true]
    }
}
