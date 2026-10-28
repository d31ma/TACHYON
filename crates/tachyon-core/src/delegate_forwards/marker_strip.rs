use super::MarkerEnd;

pub(super) struct MarkerStrip;

impl MarkerStrip {
    pub(super) fn plain(language: &str, text: &str) -> String {
        let kept = Self::kept_bytes(language, text.as_bytes());
        String::from_utf8(kept).unwrap_or_else(|_| text.to_owned())
    }

    fn kept_bytes(language: &str, bytes: &[u8]) -> Vec<u8> {
        let mut kept = Vec::with_capacity(bytes.len());
        let mut at = 0;
        while at < bytes.len() {
            let marker_end = MarkerEnd::at(language, bytes, at);
            kept.extend(marker_end.map_or(Some(bytes[at]), |_| None));
            at = marker_end.unwrap_or(at + 1);
        }
        kept
    }
}
