use super::piece::Piece;
use crate::Failure;
use std::path::Path;

pub(crate) struct JavaModelSchema;

impl JavaModelSchema {
    pub(super) const CLASS: [&'static str; 1] = ["Checked"];
    pub(super) const FIELDS: [&'static str; 5] = ["Required", "NotEmpty", "Length", "Min", "Max"];

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
        leading
            .text
            .match_indices('@')
            .any(|(at, _)| Piece::last_segment(leading.from(at + 1).path()) == Self::CLASS[0])
    }

    pub(crate) fn require_realm(path: &Path, source: &str) -> Result<(), Failure> {
        let masked = crate::annotation_layout::masked_source(path, source)?;
        let server_java = [
            path.starts_with("server/models"),
            path.extension()
                .is_some_and(|extension| extension == "java"),
        ] == [true, true];
        Self::checked_marker(Piece::new(0, &masked))
            .filter(|_| server_java)
            .map_or(Ok(()), |at| Err(Self::server_failure(path, at)))
    }

    fn checked_marker(code: Piece<'_>) -> Option<usize> {
        code.text.match_indices('@').find_map(|(at, _)| {
            let name = Piece::last_segment(code.from(at + 1).path());
            (name == Self::CLASS[0]).then_some(code.at + at)
        })
    }

    fn server_failure(path: &Path, at: usize) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        crate::Failure::one(crate::failure::diagnostic(
            1310, "Checked Java models in the server realm need the Yon schema metadata adapter",
            Some("Plain Java DTOs remain supported in server/models. Java Checked DTO metadata is currently qualified only in Tac; see docs/queue/NEXT/JAVA-YON-CHECKED-MODELS.md.".to_owned()),
            crate::failure::source_span(&portable, at, at + 1),
        ))
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
        for byte in bytes {
            *byte = [b' ', *byte][usize::from(b"\n\r".contains(byte))];
        }
    }
}
