use super::edits::TextEdit;
use crate::template::BracedAttribute;
use html5gum::{DefaultEmitter, Token, Tokenizer};

#[derive(Default)]
pub(super) struct BracedAttributeEdits {
    pub(super) edits: Vec<TextEdit>,
    pub(super) steps: Vec<(String, String)>,
}

impl BracedAttributeEdits {
    pub(super) fn plan_view(source: &str) -> Self {
        let mut plan = Self::default();
        let mut inside_style = false;
        let emitter = DefaultEmitter::<usize>::new_with_span();
        for token in Tokenizer::new_with_emitter(source, emitter).flatten() {
            match token {
                Token::StartTag(tag) if !inside_style => {
                    inside_style = tag.name.as_slice() == b"style";
                    for (name, value) in &tag.attributes {
                        let name = String::from_utf8_lossy(name);
                        let text = String::from_utf8_lossy(&value.value);
                        plan.add(source, &name, &text, value.span.start);
                    }
                }
                Token::EndTag(tag) if tag.name.as_slice() == b"style" => inside_style = false,
                _ => {}
            }
        }
        plan
    }

    fn add(&mut self, source: &str, name: &str, value: &str, start: usize) {
        let Some(braced) = BracedAttribute::find(name, value) else {
            return;
        };
        match Self::edit(source, &braced, name, value, start) {
            Some(edit) => self.edits.push(edit),
            None => self.steps.push((braced.message(), braced.help())),
        }
    }

    fn edit(
        source: &str,
        braced: &BracedAttribute,
        name: &str,
        value: &str,
        start: usize,
    ) -> Option<TextEdit> {
        let expression = braced.whole_value()?;
        if braced.is_boolean() || expression.contains('"') {
            return None;
        }
        let written = source.get(start..)?;
        let quote = written
            .strip_prefix(name)?
            .strip_prefix('=')?
            .chars()
            .next()?;
        if !matches!(quote, '"' | '\'') {
            return None;
        }
        let expected = format!("{name}={quote}{value}{quote}");
        if !written.starts_with(&expected) {
            return None;
        }
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
