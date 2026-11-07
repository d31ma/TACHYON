use crate::route_types::{Shape, TypeText};

pub(crate) struct Publications<'a> {
    language: &'a str,
    source: &'a str,
    code: Vec<bool>,
}

struct Call {
    open: usize,
    argument: String,
}

impl Publications<'_> {
    const CALL: &'static str = ".publishEvent(";
    const CALLS: [&'static str; 3] = [".publishEvent(", ".publish_event(", ".PublishEvent("];
    const STRING_NAME_REFUSAL: &'static str = "publishEvent takes the event model, not a name (ADR 0055): write publishEvent(placed) where placed is declared with a model type. `ty migrate fix --write` converts a call whose name matches a model";

    pub(crate) fn name_events(language: &str, source: &str) -> Result<String, (usize, String)> {
        let publications = Publications {
            language,
            source,
            code: crate::lexical::code_mask(language, source).code,
        };
        publications.rewrite()
    }

    pub(crate) fn event_names(language: &str, source: &str) -> Vec<String> {
        let publications = Publications {
            language,
            source,
            code: crate::lexical::code_mask(language, source).code,
        };
        let calls = publications.calls();
        calls
            .iter()
            .filter_map(|call| publications.model_of(call).ok())
            .collect()
    }

    pub(crate) fn refuse_names(language: &str, source: &str) -> Result<(), (usize, String)> {
        let publications = Publications {
            language,
            source,
            code: crate::lexical::code_mask(language, source).code,
        };
        for call in Self::CALLS {
            for (position, _) in source.match_indices(call) {
                if !publications.is_code(position) {
                    continue;
                }
                let open = position + call.len();
                let argument = publications.first_argument(open);
                if argument.starts_with(['\'', '"', '`']) {
                    return Err((open, String::from(Self::STRING_NAME_REFUSAL)));
                }
            }
        }
        Ok(())
    }

    fn rewrite(&self) -> Result<String, (usize, String)> {
        let mut output = String::with_capacity(self.source.len());
        let mut copied = 0;
        for call in self.calls() {
            let model = self.model_of(&call)?;
            output.push_str(&self.source[copied..call.open]);
            output.push('"');
            output.push_str(&model);
            output.push_str("\", ");
            copied = call.open;
        }
        output.push_str(&self.source[copied..]);
        Ok(output)
    }

    fn calls(&self) -> Vec<Call> {
        let mut calls = Vec::new();
        for (position, _) in self.source.match_indices(Self::CALL) {
            if !self.is_code(position) {
                continue;
            }
            let open = position + Self::CALL.len();
            calls.push(Call {
                open,
                argument: self.first_argument(open),
            });
        }
        calls
    }

    fn is_code(&self, position: usize) -> bool {
        self.code.get(position).copied().unwrap_or(false)
    }

    fn first_argument(&self, open: usize) -> String {
        let mut depth = 0_i32;
        for (offset, character) in self.source[open..].char_indices() {
            let position = open + offset;
            if !self.is_code(position) {
                continue;
            }
            match character {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' if depth > 0 => depth -= 1,
                ',' | ')' if depth == 0 => return self.source[open..position].trim().to_owned(),
                _ => {}
            }
        }
        self.source[open..].trim().to_owned()
    }

    fn model_of(&self, call: &Call) -> Result<String, (usize, String)> {
        let argument = call.argument.as_str();
        if argument.starts_with(['\'', '"', '`']) {
            return Err((call.open, String::from(Self::STRING_NAME_REFUSAL)));
        }
        if let Some(model) = self.cast_model(argument) {
            return Ok(model);
        }
        let (is_field, name) = match argument.strip_prefix("this.") {
            Some(field) => (true, field),
            None => (false, argument),
        };
        let type_text = if is_identifier(name) {
            self.declared_type(name, call.open, is_field)
        } else {
            None
        };
        let model = type_text.and_then(|text| self.model_name(&text));
        model.ok_or_else(|| (call.open, self.unknown_type_message(argument)))
    }

    fn cast_model(&self, argument: &str) -> Option<String> {
        if self.language == "js" {
            let rest = argument.strip_prefix("/**")?;
            let (type_text, _) = braced_type(rest.trim_start().strip_prefix("@type")?)?;
            return self.model_name(&type_text);
        }
        for keyword in [" satisfies ", " as "] {
            if let Some((_, type_text)) = argument.rsplit_once(keyword) {
                return self.model_name(type_text.trim());
            }
        }
        None
    }

    fn unknown_type_message(&self, argument: &str) -> String {
        let example = if self.language == "js" {
            "a JSDoc `@type {OrderPlaced}` or `@param {OrderPlaced} placed`, or write publishEvent(/** @type {OrderPlaced} */ (value))"
        } else {
            "`const placed: OrderPlaced = {...}`, a parameter `placed: OrderPlaced`, or publishEvent(value satisfies OrderPlaced)"
        };
        format!(
            "publishEvent({argument}) needs a variable declared with a model type, so the compiler knows the event name: use {example}"
        )
    }

    fn model_name(&self, type_text: &str) -> Option<String> {
        match TypeText::shape(self.language, type_text) {
            Ok(Shape::Model(name)) => Some(name),
            _ => None,
        }
    }

    fn declared_type(&self, name: &str, before: usize, is_field: bool) -> Option<String> {
        let limit = if is_field { self.source.len() } else { before };
        let mut found = None;
        let candidates = if self.language == "js" {
            self.documented_candidates(name, limit)
        } else {
            self.annotated_candidates(name, limit)
        };
        for (position, type_text) in candidates {
            if self.model_name(&type_text).is_some() {
                found = Some((position, type_text));
            }
        }
        found.map(|(_, type_text)| type_text)
    }

    fn annotated_candidates(&self, name: &str, limit: usize) -> Vec<(usize, String)> {
        let mut candidates = Vec::new();
        for (position, _) in self.source[..limit].match_indices(name) {
            if !self.is_code(position) || !self.is_whole_word(position, name.len()) {
                continue;
            }
            let after = &self.source[position + name.len()..limit];
            if let Some(type_text) = annotation_after_name(after) {
                candidates.push((position, type_text));
            }
        }
        candidates
    }

    fn documented_candidates(&self, name: &str, limit: usize) -> Vec<(usize, String)> {
        let mut candidates = Vec::new();
        for tag in ["@param", "@type"] {
            for (position, _) in self.source[..limit].match_indices(tag) {
                let rest = &self.source[position + tag.len()..limit];
                let Some((type_text, after_type)) = braced_type(rest) else {
                    continue;
                };
                let described = if tag == "@param" {
                    parameter_name(after_type)
                } else {
                    declared_name_after_comment(after_type)
                };
                if described == name {
                    candidates.push((position, type_text));
                }
            }
        }
        candidates.sort_by_key(|(position, _)| *position);
        candidates
    }

    fn is_whole_word(&self, position: usize, length: usize) -> bool {
        let before = self.source[..position].chars().next_back();
        let after = self.source[position + length..].chars().next();
        let is_word =
            |character: char| character.is_alphanumeric() || matches!(character, '_' | '$');
        let joined_before = before.is_some_and(|character| is_word(character) || character == '.');
        !joined_before && !after.is_some_and(is_word)
    }
}

fn is_identifier(text: &str) -> bool {
    let mut characters = text.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    let is_start = first.is_alphabetic() || matches!(first, '_' | '$');
    is_start
        && characters.all(|character| character.is_alphanumeric() || matches!(character, '_' | '$'))
}

fn annotation_after_name(after: &str) -> Option<String> {
    let rest = after.trim_start();
    let rest = rest.strip_prefix(['?', '!']).unwrap_or(rest).trim_start();
    let rest = rest.strip_prefix(':')?;
    if rest.starts_with(':') {
        return None;
    }
    let mut depth = 0_i32;
    for (offset, character) in rest.char_indices() {
        match character {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' if depth > 0 => depth -= 1,
            ',' | ')' | '=' | ';' | '{' | '\n' if depth == 0 => {
                return Some(rest[..offset].trim().to_owned());
            }
            _ => {}
        }
    }
    None
}

fn braced_type(text: &str) -> Option<(String, &str)> {
    let rest = text.trim_start().strip_prefix('{')?;
    let close = rest.find('}')?;
    Some((rest[..close].trim().to_owned(), &rest[close + 1..]))
}

fn parameter_name(after_type: &str) -> &str {
    let word = after_type.split_whitespace().next().unwrap_or("");
    let word = word.trim_start_matches('[').trim_end_matches(']');
    word.split('=').next().unwrap_or("")
}

fn declared_name_after_comment(after_type: &str) -> &str {
    let Some(close) = after_type.find("*/") else {
        return "";
    };
    let mut rest = after_type[close + 2..].trim_start();
    for keyword in ["const ", "let ", "var ", "static "] {
        if let Some(stripped) = rest.strip_prefix(keyword) {
            rest = stripped.trim_start();
        }
    }
    let end = rest
        .find(|character: char| !(character.is_alphanumeric() || matches!(character, '_' | '$')))
        .unwrap_or(rest.len());
    &rest[..end]
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::Publications;

    fn named(language: &str, source: &str) -> String {
        Publications::name_events(language, source).expect("named")
    }

    fn refused(language: &str, source: &str) -> String {
        Publications::name_events(language, source)
            .expect_err("refused")
            .1
    }

    #[test]
    fn a_typescript_call_gets_the_declared_model_name() {
        let source = "class Orders {\n  place(order: OrderPlaced): void {\n    this.applicationEventPublisher.publishEvent(order, true)\n  }\n}";
        let output = named("ts", source);
        assert!(
            output.contains("publishEvent(\"OrderPlaced\", order, true)"),
            "{output}"
        );
        let local = "class Orders {\n  place(): void {\n    const placed: OrderPlaced = { id: 'a' }\n    this.publisher.publishEvent(placed)\n  }\n}";
        assert!(named("ts", local).contains("publishEvent(\"OrderPlaced\", placed)"));
        let field = "class Orders {\n  last!: OrderPlaced\n  again(): void { this.publisher.publishEvent(this.last) }\n}";
        assert!(named("ts", field).contains("publishEvent(\"OrderPlaced\", this.last)"));
    }

    #[test]
    fn a_javascript_call_reads_the_jsdoc_type() {
        let source = "class Orders {\n  /** @param {OrderPlaced} order */\n  place(order) { this.applicationEventPublisher.publishEvent(order); }\n}";
        assert!(named("js", source).contains("publishEvent(\"OrderPlaced\", order)"));
        let local = "class Orders {\n  place() {\n    /** @type {OrderPlaced} */\n    const placed = { id: 'a' };\n    this.applicationEventPublisher.publishEvent(placed, true);\n  }\n}";
        assert!(named("js", local).contains("publishEvent(\"OrderPlaced\", placed, true)"));
    }

    #[test]
    fn a_cast_names_the_model_of_any_expression() {
        let typescript = "class A {\n  go(): void { this.p.publishEvent({ id: 'a' } satisfies OrderPlaced, true) }\n}";
        assert!(
            named("ts", typescript)
                .contains("publishEvent(\"OrderPlaced\", { id: 'a' } satisfies OrderPlaced, true)")
        );
        let javascript = "class A {\n  go(value) { this.p.publishEvent(/** @type {OrderPlaced} */ (value)); }\n}";
        assert!(
            named("js", javascript)
                .contains("publishEvent(\"OrderPlaced\", /** @type {OrderPlaced} */ (value))")
        );
    }

    #[test]
    fn names_and_untyped_values_are_refused() {
        let message = refused(
            "ts",
            "class A {\n  go(): void { this.p.publishEvent('order.placed', {}) }\n}",
        );
        assert!(
            message.contains("publishEvent takes the event model, not a name"),
            "{message}"
        );
        let message = refused(
            "js",
            "class A {\n  go(value) { this.p.publishEvent(value); }\n}",
        );
        assert!(
            message.contains("needs a variable declared with a model type"),
            "{message}"
        );
        let message = refused(
            "ts",
            "class A {\n  go(value: string): void { this.p.publishEvent(value) }\n}",
        );
        assert!(message.contains("publishEvent(value)"), "{message}");
    }

    #[test]
    fn other_languages_refuse_a_name_as_the_first_argument() {
        for (language, source) in [
            (
                "py",
                "self.application_event_publisher.publish_event('order.placed', order)\n",
            ),
            ("java", "publisher.publishEvent(\"order.placed\", order);\n"),
            (
                "go",
                "service.Publisher.PublishEvent(\"order.placed\", order)\n",
            ),
        ] {
            let refused = Publications::refuse_names(language, source).expect_err(language);
            assert!(refused.1.contains("not a name"), "{language}");
        }
        assert!(Publications::refuse_names("kt", "publisher.publishEvent(order)\n").is_ok());
    }

    #[test]
    fn comments_and_strings_are_left_alone() {
        let source =
            "// this.p.publishEvent('x', 1)\nconst text = '.publishEvent(1)';\nclass A {}\n";
        assert_eq!(named("ts", source), source);
    }
}
