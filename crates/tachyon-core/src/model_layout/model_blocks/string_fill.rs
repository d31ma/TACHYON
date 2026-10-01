pub(crate) struct StringFill;

impl StringFill {
    const QUOTES: &'static [u8] = b"\"'`";
    const FILL: u8 = b'_';

    pub(crate) fn apply(extension: &str, source: &str, masked: &str) -> String {
        let mask = crate::lexical::code_mask(extension, source);
        let original = source.as_bytes();
        let mut bytes = masked.as_bytes().to_vec();
        Self::fill_runs(&mut bytes, original, &mask.code);
        String::from_utf8(bytes).unwrap_or_default()
    }

    fn fill_runs(bytes: &mut [u8], original: &[u8], code: &[bool]) {
        for (start, end) in Self::string_runs(original, code) {
            Self::fill(&mut bytes[start..end], &original[start..end]);
        }
    }

    fn string_runs(source: &[u8], code: &[bool]) -> Vec<(usize, usize)> {
        let length = code.len().min(source.len());
        (0..length)
            .filter(|index| Self::starts_run(code, *index))
            .filter(|start| Self::QUOTES.contains(&source[*start]))
            .map(|start| (start, Self::run_end(code, start, length)))
            .collect()
    }

    fn starts_run(code: &[bool], index: usize) -> bool {
        let hidden = !code[index];
        let after_code = index.checked_sub(1).is_none_or(|previous| code[previous]);
        [hidden, after_code] == [true, true]
    }

    fn run_end(code: &[bool], start: usize, length: usize) -> usize {
        code[start..length]
            .iter()
            .position(|visible| *visible)
            .map_or(length, |run| start + run)
    }

    fn fill(bytes: &mut [u8], original: &[u8]) {
        bytes
            .iter_mut()
            .zip(original)
            .filter(|(_, written)| !written.is_ascii_whitespace())
            .for_each(|(byte, _)| *byte = Self::FILL);
    }
}
