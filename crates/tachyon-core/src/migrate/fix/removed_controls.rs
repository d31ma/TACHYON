use super::edits::TextEdit;
use html5gum::{DefaultEmitter, StartTag, Token, Tokenizer};

#[derive(Default)]
pub(super) struct RemovedControlEdits {
    pub(super) edits: Vec<TextEdit>,
}

impl RemovedControlEdits {
    const START_TAGS: [(&'static str, &'static str); 3] =
        [("if", "logic"), ("else", "logic else"), ("for", "loop")];
    const END_TAGS: [(&'static str, &'static str); 3] =
        [("if", "logic"), ("else", "logic"), ("for", "loop")];
    const ATTRIBUTES: [(&'static str, &'static str); 2] = [(":when", ":if"), (":each", ":for")];
    const OLD_SEPARATOR: &'static str = " in ";
    const NEW_SEPARATOR: &'static str = " of ";
    const COUNTED_LOOP_MARK: &'static str = ";";
    const START_TAG_OPENING: usize = "<".len();
    const END_TAG_OPENING: usize = "</".len();

    pub(super) fn plan_view(source: &str) -> Self {
        let mut plan = Self::default();
        let emitter = DefaultEmitter::<usize>::new_with_span();
        for token in Tokenizer::new_with_emitter(source, emitter).flatten() {
            match token {
                Token::StartTag(tag) => plan.add_start_tag(source, &tag),
                Token::EndTag(tag) => {
                    let name = String::from_utf8_lossy(&tag.name);
                    let start = tag.span.start + Self::END_TAG_OPENING;
                    plan.rename(source, start, &name, &Self::END_TAGS);
                }
                _ => {}
            }
        }
        plan
    }

    fn add_start_tag(&mut self, source: &str, tag: &StartTag<usize>) {
        let name = String::from_utf8_lossy(&tag.name);
        let start = tag.span.start + Self::START_TAG_OPENING;
        if !self.rename(source, start, &name, &Self::START_TAGS) {
            return;
        }
        for (attribute, value) in &tag.attributes {
            let attribute = String::from_utf8_lossy(attribute);
            self.rename(source, value.span.start, &attribute, &Self::ATTRIBUTES);
        }
        let each = tag.attributes.get(b":each".as_slice());
        if let Some(each) = each.filter(|each| Self::is_list_loop(&each.value)) {
            self.add_separator(source, each.span.start, tag.span.end);
        }
    }

    fn rename(&mut self, source: &str, start: usize, name: &str, renames: &[(&str, &str)]) -> bool {
        let Some((_, canonical)) = renames.iter().find(|(old, _)| *old == name) else {
            return false;
        };
        let end = start + name.len();
        let written = source.get(start..end).unwrap_or_default();
        if !written.eq_ignore_ascii_case(name) {
            return false;
        }
        self.edits.push(TextEdit::replace(start, end, canonical));
        true
    }

    fn is_list_loop(value: &[u8]) -> bool {
        let value = String::from_utf8_lossy(value);
        !value.contains(Self::COUNTED_LOOP_MARK) && value.contains(Self::OLD_SEPARATOR)
    }

    fn add_separator(&mut self, source: &str, attribute_start: usize, tag_end: usize) {
        let written = source.get(attribute_start..tag_end).unwrap_or_default();
        if let Some(offset) = written.find(Self::OLD_SEPARATOR) {
            let start = attribute_start + offset;
            let end = start + Self::OLD_SEPARATOR.len();
            self.edits
                .push(TextEdit::replace(start, end, Self::NEW_SEPARATOR));
        }
    }
}

#[cfg(test)]
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
