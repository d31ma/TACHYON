use super::edits::TextEdit;
use crate::template::BracedAttribute;
use html5gum::{DefaultEmitter, StartTag, Token, Tokenizer};

#[derive(Default)]
pub(super) struct BracedAttributeEdits {
    pub(super) edits: Vec<TextEdit>,
    pub(super) steps: Vec<(String, String)>,
}

impl BracedAttributeEdits {
    const QUOTES: [char; 2] = ['"', '\''];

    pub(super) fn plan_view(source: &str) -> Self {
        let mut plan = Self::default();
        plan.read_tags(source);
        plan
    }

    fn read_tags(&mut self, source: &str) {
        let mut inside_style = false;
        let emitter = DefaultEmitter::<usize>::new_with_span();
        for token in Tokenizer::new_with_emitter(source, emitter).flatten() {
            inside_style = self.read_token(source, token, inside_style);
        }
    }

    fn read_token(&mut self, source: &str, token: Token<usize>, inside_style: bool) -> bool {
        match token {
            Token::StartTag(tag) if !inside_style => {
                self.add_attributes(source, &tag);
                tag.name.as_slice() == b"style"
            }
            Token::EndTag(tag) if tag.name.as_slice() == b"style" => false,
            _ => inside_style,
        }
    }

    fn add_attributes(&mut self, source: &str, tag: &StartTag<usize>) {
        for (name, value) in &tag.attributes {
            let name = String::from_utf8_lossy(name);
            let text = String::from_utf8_lossy(&value.value);
            self.add(source, &name, &text, value.span.start);
        }
    }

    fn add(&mut self, source: &str, name: &str, value: &str, start: usize) {
        if let Some(braced) = BracedAttribute::find(name, value) {
            self.add_braced(source, &braced, name, value, start);
        }
    }

    fn add_braced(
        &mut self,
        source: &str,
        braced: &BracedAttribute,
        name: &str,
        value: &str,
        start: usize,
    ) {
        let edit = Self::edit(source, braced, name, value, start);
        let step = edit.is_none().then(|| (braced.message(), braced.help()));
        self.edits.extend(edit);
        self.steps.extend(step);
    }

    fn edit(
        source: &str,
        braced: &BracedAttribute,
        name: &str,
        value: &str,
        start: usize,
    ) -> Option<TextEdit> {
        let expression = braced.whole_value()?;
        let rejected = [braced.is_boolean(), expression.contains('"')];
        (rejected == [false, false]).then_some(())?;
        let written = source.get(start..)?;
        let quote = written
            .strip_prefix(name)?
            .strip_prefix('=')?
            .chars()
            .next()?;
        Self::QUOTES.contains(&quote).then_some(())?;
        let expected = format!("{name}={quote}{value}{quote}");
        written.starts_with(&expected).then_some(())?;
        let replacement = format!(":{name}=\"{expression}\"");
        Some(TextEdit::replace(
            start,
            start + expected.len(),
            &replacement,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::BracedAttributeEdits;
    use crate::migrate::fix::edits::TextEdit;

    fn converted(source: &str) -> String {
        TextEdit::apply(source, &BracedAttributeEdits::plan_view(source).edits)
    }

    #[test]
    fn whole_value_braces_become_dynamic_attributes() {
        assert_eq!(
            converted("<p title=\"{name}\" class='{ theme.kind }'>{name}</p>\n"),
            "<p :title=\"name\" :class=\"theme.kind\">{name}</p>\n"
        );
    }

    #[test]
    fn mixed_text_boolean_and_literal_braces_stay_for_a_person() {
        for source in [
            "<p class=\"card {kind}\">x</p>",
            "<div hidden=\"{drawer.open ? '' : 'hidden'}\"></div>",
            "<input pattern=\"[0-9]{3}\" />",
            "<p title='{label == \"x\"}'>x</p>",
            "<p :title=\"name\">x</p>",
            "<style>p[title=\"{name}\"] { color: red; }</style>",
        ] {
            assert_eq!(converted(source), source);
        }
        let plan = BracedAttributeEdits::plan_view("<p class=\"card {kind}\">x</p>");
        assert_eq!(
            plan.steps,
            vec![(
                String::from("The attribute class uses braces, but braces work only in text."),
                String::from("Write :class=\"'card ' + kind\" to join the text and the values."),
            )]
        );
    }
}
