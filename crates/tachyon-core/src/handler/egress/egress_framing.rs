use super::EgressHead;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EgressFraming {
    Length(u64),
    Chunked,
}

impl EgressFraming {
    const TRANSFER_ENCODING: &'static str = "transfer-encoding";
    const CONTENT_LENGTH: &'static str = "content-length";
    const CHUNKED: &'static str = "chunked";
    const LIST_SEPARATOR: char = ',';

    pub(super) fn read(fields: &str) -> Option<Self> {
        let encodings = EgressHead::values(fields, Self::TRANSFER_ENCODING)?;
        let lengths = EgressHead::values(fields, Self::CONTENT_LENGTH)?;
        Self::choose(&encodings, &lengths)
    }

    fn choose(encodings: &[String], lengths: &[String]) -> Option<Self> {
        if encodings.is_empty() {
            return Self::length(lengths);
        }
        Self::encoded(encodings, lengths)
    }

    fn encoded(encodings: &[String], lengths: &[String]) -> Option<Self> {
        lengths.is_empty().then_some(())?;
        Self::chunked(encodings)
    }

    fn chunked(encodings: &[String]) -> Option<Self> {
        let last = Self::parts(encodings)
            .into_iter()
            .rfind(|item| !item.is_empty())?;
        last.eq_ignore_ascii_case(Self::CHUNKED)
            .then_some(Self::Chunked)
    }

    fn length(lengths: &[String]) -> Option<Self> {
        let values: Vec<u64> = Self::parts(lengths)
            .into_iter()
            .map(Self::number)
            .collect::<Option<Vec<u64>>>()?;
        Self::single(&values)
    }

    fn parts(values: &[String]) -> Vec<&str> {
        values
            .iter()
            .flat_map(|value| value.split(Self::LIST_SEPARATOR))
            .map(str::trim)
            .collect()
    }

    fn number(text: &str) -> Option<u64> {
        Self::digits(text).then(|| text.parse().ok()).flatten()
    }

    fn digits(text: &str) -> bool {
        text.bytes().all(|byte| byte.is_ascii_digit())
    }

    fn single(values: &[u64]) -> Option<Self> {
        let first = values.first().copied().unwrap_or(0);
        values
            .iter()
            .all(|value| *value == first)
            .then_some(Self::Length(first))
    }
}
