use std::collections::BTreeSet;
use std::ops::Range;
use std::path::Path;

pub(super) struct DartDelegateBases {
    code: String,
    services: BTreeSet<String>,
}

impl DartDelegateBases {
    const DELEGATE: &'static str = "@Delegate";
    const CLASS: &'static str = "class ";
    const OPEN: char = '{';
    const NEEDS: &'static str = "@Needs";
    const CALL: &'static str = "()";
    const STATEMENT_END: char = ';';
    const FINAL: &'static str = "final ";
    const LATE: &'static str = "late ";
    const MODIFIERS: [&'static str; 2] = ["final", "late"];
    const SERVICES: &'static str = "client/services";
    const EXTENSION: &'static str = "dart";

    pub(super) fn late(snapshot_root: &Path, source: &str) -> String {
        let mask = crate::lexical::code_mask(Self::EXTENSION, source);
        let bases = Self {
            code: String::from_utf8_lossy(&super::masked_code(source, &mask.code)).into_owned(),
            services: Self::services(snapshot_root),
        };
        let edits: Vec<(Range<usize>, String)> = bases
            .delegate_bodies()
            .into_iter()
            .flat_map(|body| bases.base_edits(&body))
            .collect();
        edits
            .iter()
            .rev()
            .fold(source.to_owned(), |mut text, (range, replacement)| {
                text.replace_range(range.clone(), replacement);
                text
            })
    }

    fn services(snapshot_root: &Path) -> BTreeSet<String> {
        std::fs::read_dir(snapshot_root.join(Self::SERVICES))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().and_then(|value| value.to_str()) == Some(Self::EXTENSION)
            })
            .filter_map(|path| Some(path.file_stem()?.to_str()?.to_owned()))
            .collect()
    }

    fn delegate_bodies(&self) -> Vec<Range<usize>> {
        self.code
            .match_indices(Self::DELEGATE)
            .filter_map(|(at, _)| self.body_after(at))
            .collect()
    }

    fn body_after(&self, at: usize) -> Option<Range<usize>> {
        let class = at + self.code[at..].find(Self::CLASS)?;
        let open = class + self.code[class..].find(Self::OPEN)?;
        let close = super::matching(&self.code, open)?;
        Some(open + 1..close)
    }

    fn base_edits(&self, body: &Range<usize>) -> Vec<(Range<usize>, String)> {
        self.code[body.clone()]
            .match_indices(Self::NEEDS)
            .filter_map(|(at, _)| self.base_edit(body.start + at))
            .flatten()
            .collect()
    }

    fn base_edit(&self, marker_at: usize) -> Option<[(Range<usize>, String); 2]> {
        let after_marker = marker_at + Self::NEEDS.len();
        let marker_end = after_marker
            + usize::from(self.code[after_marker..].starts_with(Self::CALL)) * Self::CALL.len();
        let rest = &self.code[marker_end..];
        let start = marker_end + (rest.len() - rest.trim_start().len());
        let end = start + self.code[start..].find(Self::STATEMENT_END)?;
        self.is_base(&self.code[start..end]).then(|| {
            [
                (marker_at..marker_end, " ".repeat(marker_end - marker_at)),
                self.late_edit(start),
            ]
        })
    }

    fn is_base(&self, declaration: &str) -> bool {
        let words: Vec<&str> = declaration
            .split_whitespace()
            .filter(|word| !Self::MODIFIERS.contains(word))
            .collect();
        let kind = words
            .split_last()
            .map(|(_, kind)| kind.join(" "))
            .unwrap_or_default();
        self.services.contains(kind.trim_end_matches('?'))
    }

    fn late_edit(&self, start: usize) -> (Range<usize>, String) {
        let declaration = &self.code[start..];
        let written_final = declaration.starts_with(Self::FINAL);
        let written_late = declaration.starts_with(Self::LATE);
        let replaced = start..start + usize::from(written_final) * Self::FINAL.len();
        let late = [Self::LATE, ""][usize::from(written_late)];
        (replaced, late.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::DartDelegateBases;

    struct LateFixture;

    impl LateFixture {
        const QUOTE: &'static str = "@Service()\nclass QuoteService {\n  @Needs()\n  final TaxDelegate taxDelegate;\n\n  int quote() => taxDelegate.rate('north');\n}\n\n\
             @Delegate()\nclass TaxDelegate {\n  @Needs()\n  final TaxService taxService;\n  @Needs()\n  final LabelDelegate label;\n\n  int rate(String region) => taxService.rate(region) * 2;\n}\n";

        fn late(source: &str) -> String {
            let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
            let services = root.path().join("client/services");
            std::fs::create_dir_all(&services).unwrap_or_else(|error| panic!("services: {error}"));
            for class in ["TaxService", "QuoteService"] {
                std::fs::write(services.join(format!("{class}.dart")), "")
                    .unwrap_or_else(|error| panic!("service: {error}"));
            }
            DartDelegateBases::late(root.path(), source)
        }
    }

    #[test]
    fn a_delegate_base_field_waits_for_the_page_service() {
        let text = LateFixture::late(LateFixture::QUOTE);
        assert!(text.contains("  late TaxService taxService;"), "{text}");
        assert!(
            text.contains("  @Needs()\n  final TaxDelegate taxDelegate;"),
            "{text}"
        );
        assert!(
            text.contains("  @Needs()\n  final LabelDelegate label;"),
            "{text}"
        );
        assert_eq!(text.matches("@Needs").count(), 2, "{text}");
    }

    #[test]
    fn a_late_base_field_stays_late_and_a_bare_one_becomes_late() {
        for (written, expected) in [
            ("late TaxService taxService;", "late TaxService taxService;"),
            ("TaxService? taxService;", "late TaxService? taxService;"),
        ] {
            let source = LateFixture::QUOTE.replace("final TaxService taxService;", written);
            let text = LateFixture::late(&source);
            assert!(text.contains(&format!("  {expected}")), "{text}");
        }
    }
}
