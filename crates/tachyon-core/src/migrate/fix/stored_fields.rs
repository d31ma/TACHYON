use super::edits::TextEdit;
use super::scan::SourceText;
use crate::behavior::{SigilField, StoredState};
use std::path::Path;

#[derive(Default)]
pub(super) struct StoredFieldEdits {
    pub(super) edits: Vec<TextEdit>,
    pub(super) notes: Vec<String>,
    pub(super) clashes: Vec<String>,
}

impl StoredFieldEdits {
    pub(super) fn plan(relative: &str, source: &str) -> Self {
        let path = Path::new(relative);
        let fields = StoredState::sigil_fields(path, source);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let text = SourceText::read(extension, source).filter(|_| !fields.is_empty());
        text.map(|text| Self::plan_fields(&text, fields))
            .unwrap_or_default()
    }

    fn plan_fields(text: &SourceText<'_>, fields: Vec<SigilField>) -> Self {
        let mut result = Self::default();
        for field in fields {
            result.add_field(text, field);
        }
        result
    }

    fn add_field(&mut self, text: &SourceText<'_>, field: SigilField) {
        let source = text.source;
        let bare = field.written.trim_start_matches('$');
        if !text.code_words(bare).is_empty() {
            self.clashes.push(field.written);
            return;
        }
        let marker = StoredState::marker_for(&field.written);
        let indent = &source[field.line_start..field.line_start + field.indent];
        self.edits.push(TextEdit::insert(
            field.line_start + field.indent,
            &format!("@{marker}\n{indent}"),
        ));
        let uses = text.code_words(&field.written);
        let renamed_uses = uses
            .into_iter()
            .map(|at| TextEdit::replace(at, at + field.written.len(), bare));
        self.edits.extend(renamed_uses);
        let line = source[..field.at].matches('\n').count() + 1;
        self.notes.push(format!(
            "Changed the field `{}` on line {line} to `@{marker} {bare}`: Tac keeps fields through markers, not `$` prefixes (ADR 0059).",
            field.written
        ));
    }

    pub(super) fn plan_view(source: &str) -> Vec<TextEdit> {
        source
            .match_indices(".$")
            .filter_map(|(at, _)| Self::view_edit(source.as_bytes(), at))
            .collect()
    }

    fn view_edit(bytes: &[u8], at: usize) -> Option<TextEdit> {
        let start = at + 1;
        let sigils = 1 + usize::from(bytes.get(start + 1) == Some(&b'$'));
        let next = bytes.get(start + sigils).copied();
        let named = next.is_some_and(Self::starts_name);
        let before = at.checked_sub(1).and_then(|before| bytes.get(before));
        let owner = before.copied().is_some_and(Self::continues_name);
        ([named, owner] == [true, true]).then(|| TextEdit::delete(start, start + sigils))
    }

    fn starts_name(byte: u8) -> bool {
        [byte.is_ascii_alphabetic(), byte == b'_'].contains(&true)
    }

    fn continues_name(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }
}

#[cfg(test)]
mod tests {
    use super::StoredFieldEdits;
    use crate::migrate::fix::edits::TextEdit;

    fn converted(relative: &str, source: &str) -> String {
        TextEdit::apply(source, &StoredFieldEdits::plan(relative, source).edits)
    }

    #[test]
    fn dollar_fields_become_markers_in_javascript_typescript_and_dart() {
        assert_eq!(
            converted(
                "client/services/EditorService.js",
                "export default class EditorService {\n  $draft = ''\n  $$theme = 'light'\n  dark() { this.$$theme = 'dark' }\n}\n",
            ),
            "export default class EditorService {\n  @SessionState\n  draft = ''\n  @LocalState\n  theme = 'light'\n  dark() { this.theme = 'dark' }\n}\n",
        );
        assert_eq!(
            converted(
                "client/services/EditorService.ts",
                "export default class EditorService {\n  private $$theme: string = 'light'\n}\n",
            ),
            "export default class EditorService {\n  @LocalState\n  private theme: string = 'light'\n}\n",
        );
        assert_eq!(
            converted(
                "client/services/EditorService.dart",
                "class EditorService {\n  String $draft = '';\n}\n",
            ),
            "class EditorService {\n  @SessionState\n  String draft = '';\n}\n",
        );
    }

    #[test]
    fn a_field_whose_plain_name_is_taken_is_left_for_the_author() {
        let plan = StoredFieldEdits::plan(
            "client/services/EditorService.js",
            "export default class EditorService {\n  $$theme = 'light'\n  theme = 'dark'\n}\n",
        );
        assert!(plan.edits.is_empty());
        assert_eq!(plan.clashes, ["$$theme"]);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn a_project_fix_converts_services_and_views_once() {
        let root = tempfile::tempdir().expect("project");
        let write = |relative: &str, source: &str| {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folders");
            std::fs::write(path, source).expect("source");
        };
        write(
            "client/services/EditorService.js",
            "@Service\nexport default class EditorService {\n  $$theme = 'light'\n}\n",
        );
        write(
            "client/components/draft/editor/tac.html",
            "<button on:click=\"editor.$$theme = 'dark'\">{editor.$$theme}</button>\n",
        );
        let planned = crate::migrate::MigrationFix::plan(root.path()).expect("plan");
        planned.write(root.path()).expect("write");
        let again = crate::migrate::MigrationFix::plan(root.path()).expect("second plan");
        assert_eq!(again.change_count(), 0);
        let read =
            |relative: &str| std::fs::read_to_string(root.path().join(relative)).expect("read");
        assert_eq!(
            read("client/services/EditorService.js"),
            "@Service\nexport default class EditorService {\n  @LocalState\n  theme = 'light'\n}\n",
        );
        assert_eq!(
            read("client/components/draft/editor/tac.html"),
            "<button on:click=\"editor.theme = 'dark'\">{editor.theme}</button>\n",
        );
    }

    #[test]
    fn views_read_the_plain_field_names() {
        let view = "<input on:input=\"editor.$draft = $event.target.value\">\n<p>{editor.$$theme} costs $5.</p>\n";
        assert_eq!(
            TextEdit::apply(view, &StoredFieldEdits::plan_view(view)),
            "<input on:input=\"editor.draft = $event.target.value\">\n<p>{editor.theme} costs $5.</p>\n",
        );
    }
}
