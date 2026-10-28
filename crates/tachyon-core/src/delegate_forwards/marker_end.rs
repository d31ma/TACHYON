pub(super) struct MarkerEnd;

impl MarkerEnd {
    const ENCLOSED: [(&'static str, &'static [u8], &'static [u8]); 4] = [
        ("hpp", b"[[", b"]]"),
        ("fs", b"[<", b">]"),
        ("rs", b"#[", b"]"),
        ("cs", b"[", b"]"),
    ];

    pub(super) fn at(language: &str, bytes: &[u8], at: usize) -> Option<usize> {
        Self::annotation(bytes, at).or_else(|| {
            Self::ENCLOSED
                .iter()
                .filter(|(owner, _, _)| *owner == language)
                .find_map(|(_, open, close)| Self::enclosed(bytes, at, (*open, *close)))
        })
    }

    fn annotation(bytes: &[u8], at: usize) -> Option<usize> {
        let start = (bytes[at] == b'@').then_some(at + 1)?;
        bytes.get(start)?.is_ascii_uppercase().then_some(())?;
        let length = bytes[start..]
            .iter()
            .position(|byte| !Self::is_name_byte(*byte))
            .unwrap_or(bytes.len() - start);
        Some(Self::after_arguments(bytes, start + length))
    }

    fn after_arguments(bytes: &[u8], at: usize) -> usize {
        let opens = bytes.get(at) == Some(&b'(');
        opens
            .then(|| Self::matching_close(bytes, at))
            .flatten()
            .map_or(at, |close| close + 1)
    }

    fn matching_close(bytes: &[u8], open: usize) -> Option<usize> {
        let mut depth = 0_usize;
        bytes[open..]
            .iter()
            .position(|byte| {
                depth = (depth + usize::from(*byte == b'(')).saturating_sub(usize::from(*byte == b')'));
                [*byte == b')', depth == 0] == [true, true]
            })
            .map(|offset| open + offset)
    }

    fn enclosed(bytes: &[u8], at: usize, (open, close): (&[u8], &[u8])) -> Option<usize> {
        bytes[at..].starts_with(open).then_some(())?;
        let inside = at + open.len();
        bytes.get(inside).is_some_and(u8::is_ascii_uppercase).then_some(())?;
        let length = bytes[inside..]
            .windows(close.len())
            .position(|window| window == close)?;
        Some(inside + length + close.len())
    }

    fn is_name_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_', byte == b'.'].contains(&true)
    }
}
