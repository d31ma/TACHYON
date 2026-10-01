use super::piece::Piece;
use crate::Failure;
use std::path::Path;

pub(crate) struct JavaModelSchema;

impl JavaModelSchema {
    pub(super) const CLASS: [&'static str; 3] = ["Checked", "Settings", "Description"];
    pub(super) const FIELDS: [&'static str; 6] =
        ["Required", "NotEmpty", "Length", "Min", "Max", "Setting"];

    pub(super) fn first_refused(code: Piece<'_>, allowed: &[&str]) -> Option<usize> {
        code.text.match_indices('@').find_map(|(at, _)| {
            let name = Piece::last_segment(code.from(at + 1).path());
            let refused = [
                crate::behavior::is_framework_marker(name),
                !allowed.contains(&name),
            ] == [true, true];
            refused.then_some(code.at + at)
        })
    }

    pub(super) fn checked(code: Piece<'_>) -> bool {
        let leading = code.slice(0, code.after_annotations().at - code.at);
        leading.text.match_indices('@').any(|(at, _)| {
            ["Checked", "Settings"].contains(&Piece::last_segment(leading.from(at + 1).path()))
        })
    }

    pub(crate) fn blanked(path: &Path, source: &str) -> Result<String, Failure> {
        let masked = crate::annotation_layout::masked_source(path, source)?;
        Ok(Self::blank_source(&masked, source))
    }

    fn blank_source(masked: &str, source: &str) -> String {
        let code = Piece::new(0, masked);
        let mut bytes = source.as_bytes().to_vec();
        for (at, _) in masked.match_indices('@') {
            Self::blank_marker(code.from(at), &mut bytes);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn blank_marker(code: Piece<'_>, bytes: &mut [u8]) {
        let annotation = code.from(1);
        let name = annotation.path();
        let schema = [
            Self::CLASS.contains(&Piece::last_segment(name)),
            Self::FIELDS.contains(&Piece::last_segment(name)),
        ]
        .contains(&true);
        let rest = annotation.from(name.len()).trim();
        let end = [code.at, rest.after_arguments().at][usize::from(schema)];
        Self::blank_bytes(&mut bytes[code.at..end]);
    }

    pub(crate) fn blank_bytes(bytes: &mut [u8]) {
        Self::clear_bytes(bytes);
    }

    fn clear_bytes(bytes: &mut [u8]) {
        for byte in bytes {
            *byte = [b' ', *byte][usize::from(b"\n\r".contains(byte))];
        }
    }
}
