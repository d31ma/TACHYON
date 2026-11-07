use super::edits::TextEdit;
use html5gum::{DefaultEmitter, EndTag, StartTag, Token, Tokenizer};

pub(super) struct RemovedControlEdits {
    pub(super) edits: Vec<TextEdit>,
}

impl RemovedControlEdits {
    const START_TAGS: [(&'static str, &'static str); 3] =
        [("if", "logic"), ("else", "logic else"), ("for", "loop")];
    const END_TAGS: [(&'static str, &'static str); 3] =
        [("if", "logic"), ("else", "logic"), ("for", "loop")];
    const ATTRIBUTES: [(&'static str, &'static str); 2] = [(":when", ":if"), (":each", ":for")];
    const LOOP_ATTRIBUTE: &'static [u8] = b":each";
    const OLD_SEPARATOR: &'static str = " in ";
    const NEW_SEPARATOR: &'static str = " of ";
    const COUNTED_LOOP_MARK: &'static str = ";";
    const START_TAG_OPENING: usize = "<".len();
    const END_TAG_OPENING: usize = "</".len();

    pub(super) fn plan_view(source: &str) -> Self {
        let emitter = DefaultEmitter::<usize>::new_with_span();
        let edits = Tokenizer::new_with_emitter(source, emitter)
            .flatten()
            .flat_map(|token| Self::token_edits(source, token))
            .collect();
        Self { edits }
    }

    fn token_edits(source: &str, token: Token<usize>) -> Vec<TextEdit> {
        match token {
            Token::StartTag(tag) => Self::start_tag_edits(source, &tag),
            Token::EndTag(tag) => Self::end_tag_edits(source, &tag),
            _ => Vec::new(),
        }
    }

    fn end_tag_edits(source: &str, tag: &EndTag<usize>) -> Vec<TextEdit> {
        let name = String::from_utf8_lossy(&tag.name);
        let start = tag.span.start + Self::END_TAG_OPENING;
        Self::renamed(source, start, &name, &Self::END_TAGS)
            .into_iter()
            .collect()
    }

    fn start_tag_edits(source: &str, tag: &StartTag<usize>) -> Vec<TextEdit> {
        let name = String::from_utf8_lossy(&tag.name);
        let start = tag.span.start + Self::START_TAG_OPENING;
        Self::renamed(source, start, &name, &Self::START_TAGS)
            .map(|tag_edit| Self::with_attribute_edits(source, tag, tag_edit))
            .unwrap_or_default()
    }

    fn with_attribute_edits(
        source: &str,
        tag: &StartTag<usize>,
        tag_edit: TextEdit,
    ) -> Vec<TextEdit> {
        let attribute_edits = tag.attributes.iter().filter_map(|(attribute, value)| {
            let attribute = String::from_utf8_lossy(attribute);
            Self::renamed(source, value.span.start, &attribute, &Self::ATTRIBUTES)
        });
        let separator_edit = tag
            .attributes
            .get(Self::LOOP_ATTRIBUTE)
            .filter(|each| Self::is_list_loop(&each.value))
            .and_then(|each| Self::separator_edit(source, each.span.start, tag.span.end));
        std::iter::once(tag_edit)
            .chain(attribute_edits)
            .chain(separator_edit)
            .collect()
    }

    fn renamed(
        source: &str,
        start: usize,
        name: &str,
        renames: &[(&str, &str)],
    ) -> Option<TextEdit> {
        let (_, canonical) = renames.iter().find(|(old, _)| *old == name)?;
        let end = start + name.len();
        source
            .get(start..end)
            .filter(|written| written.eq_ignore_ascii_case(name))?;
        Some(TextEdit::replace(start, end, canonical))
    }

    fn is_list_loop(value: &[u8]) -> bool {
        let value = String::from_utf8_lossy(value);
        let marks = [
            value.contains(Self::COUNTED_LOOP_MARK),
            value.contains(Self::OLD_SEPARATOR),
        ];
        marks == [false, true]
    }

    fn separator_edit(source: &str, attribute_start: usize, tag_end: usize) -> Option<TextEdit> {
        let offset = source
            .get(attribute_start..tag_end)?
            .find(Self::OLD_SEPARATOR)?;
        let start = attribute_start + offset;
        let end = start + Self::OLD_SEPARATOR.len();
        Some(TextEdit::replace(start, end, Self::NEW_SEPARATOR))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::RemovedControlEdits;
    use crate::migrate::fix::MigrationFix;
    use crate::migrate::fix::edits::TextEdit;
    use std::fs;
    use tempfile::TempDir;

    fn converted(source: &str) -> String {
        TextEdit::apply(source, &RemovedControlEdits::plan_view(source).edits)
    }

    #[test]
    fn if_and_else_become_logic_branches() {
        assert_eq!(
            converted("<if :when=\"cart.ready\"><p>Ready</p></if>\n<else><p>Wait</p></else>\n"),
            "<logic :if=\"cart.ready\"><p>Ready</p></logic>\n<logic else><p>Wait</p></logic>\n"
        );
    }

    #[test]
    fn for_each_in_becomes_loop_for_of() {
        assert_eq!(
            converted("<ul><for :each=\"const item in cart.items\"><li>{item}</li></for></ul>"),
            "<ul><loop :for=\"const item of cart.items\"><li>{item}</li></loop></ul>"
        );
    }

    #[test]
    fn counted_for_keeps_its_declaration() {
        assert_eq!(
            converted("<for :each=\"let i = 0; i &lt; 2; i++\"><b>{i}</b></for>"),
            "<loop :for=\"let i = 0; i &lt; 2; i++\"><b>{i}</b></loop>"
        );
    }

    #[test]
    fn canonical_controls_comments_and_other_tags_stay_unchanged() {
        let source = "<logic :if=\"a\"><p>in</p></logic><loop :for=\"x of xs\">{x}</loop>\
                      <!-- <if :when=\"a\"></if> --><iframe title=\"in\"></iframe>\
                      <case :when=\"'a'\">a</case><form-field></form-field>";
        assert_eq!(converted(source), source);
    }

    #[test]
    fn migrate_fix_writes_the_rewritten_component_view() {
        let project = TempDir::new().expect("project");
        let view = project
            .path()
            .join("client/components/cart/summary/tac.html");
        fs::create_dir_all(view.parent().expect("parent")).expect("folders");
        fs::write(
            &view,
            "<if :when=\"cart.ready\"><p>Ready</p></if><else><p>Wait</p></else>\n",
        )
        .expect("view");
        MigrationFix::plan(project.path())
            .expect("plan")
            .write(project.path())
            .expect("write");
        assert_eq!(
            fs::read_to_string(&view).expect("view"),
            "<logic :if=\"cart.ready\"><p>Ready</p></logic><logic else><p>Wait</p></logic>\n"
        );
    }
}
