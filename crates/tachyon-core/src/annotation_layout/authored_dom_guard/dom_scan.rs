use super::dom_bindings::DomBindings;
use super::dom_expression::DomExpression;
use super::dom_kind::DomKind;
use super::dom_statement::DomStatement;
use super::dom_target::DomTarget;
use crate::Failure;
use crate::annotation_layout::single_class::Tokens;
use std::ops::Range;
use std::path::Path;

pub(super) struct DomScan<'s> {
    tokens: Tokens<'s>,
    bindings: DomBindings,
}

impl<'s> DomScan<'s> {
    const HTML_WRITES: [&'static str; 2] = ["innerHTML", "outerHTML"];
    const HTML_CALLS: [&'static str; 3] = ["insertAdjacentHTML", "setHTMLUnsafe", "setHTML"];
    const STYLE_WRITES: [&'static str; 3] = ["cssText", "adoptedStyleSheets", "style"];
    const INSERTIONS: [&'static str; 9] = [
        "append",
        "appendChild",
        "prepend",
        "before",
        "after",
        "replaceChildren",
        "replaceChild",
        "insertBefore",
        "insertAdjacentElement",
    ];
    const DECLARATIONS: [&'static str; 3] = ["const", "let", "var"];
    const ASSIGNMENT_PREFIX: [&'static str; 9] = ["+", "-", "*", "/", "%", "&", "|", "?", "^"];

    pub(super) fn new(path: &'s Path, source: &'s str) -> Result<Self, Failure> {
        let tokens = Tokens::read_script_source(path, source)?;
        Ok(Self {
            tokens,
            bindings: DomBindings::new(),
        })
    }

    pub(super) fn first_sink(&mut self) -> Option<Range<usize>> {
        self.scan()
    }

    fn scan(&mut self) -> Option<Range<usize>> {
        (0..self.tokens.items.len()).find_map(|at| {
            self.update_bindings(at);
            self.sink(at)
        })
    }

    fn text(&self, at: usize) -> &str {
        self.tokens.items.get(at).map_or("", |token| token.text)
    }

    fn update_bindings(&mut self, at: usize) {
        let text = self.text_owned(at);
        Self::update_scope(&mut self.bindings, &text);
        self.assignment(at)
            .into_iter()
            .for_each(|(name, start, declaration, compound)| {
                let reader = DomExpression::new(&self.tokens.items, &self.bindings);
                let end = DomStatement::value_end(&self.tokens, start);
                let value = reader.kind(start, end);
                let kind = self.bindings.kind(&name).assigned(value, compound);
                self.bind(name, kind, declaration);
            });
    }

    fn bind(&mut self, name: String, kind: DomKind, declaration: bool) {
        if name.contains('.') {
            self.bindings.assign_member(name, kind);
            return;
        }
        self.bindings.assign(name, kind, declaration);
    }

    fn text_owned(&self, at: usize) -> String {
        self.text(at).to_owned()
    }

    fn update_scope(bindings: &mut DomBindings, text: &str) {
        match text {
            "{" => bindings.enter(),
            "}" => bindings.leave(),
            _ => {}
        }
    }

    fn assignment(&self, at: usize) -> Option<(String, usize, bool, bool)> {
        Self::plain_assignment(&self.tokens, at).then_some(())?;
        let operator = self.operator_start(at);
        let name_at = self.assignment_name_at(operator)?;
        let token = self.tokens.items.get(name_at)?;
        (!token.literal).then_some(())?;
        let name = DomTarget::new(&self.tokens.items).path(name_at)?;
        let declaration = name_at
            .checked_sub(1)
            .is_some_and(|before| Self::DECLARATIONS.contains(&self.text(before)));
        Some((name, at + 1, declaration, operator < at))
    }

    fn operator_start(&self, at: usize) -> usize {
        (0..at)
            .rev()
            .find(|before| !Self::ASSIGNMENT_PREFIX.contains(&self.text(*before)))
            .map_or(0, |before| before + 1)
    }

    fn assignment_name_at(&self, operator: usize) -> Option<usize> {
        let previous = operator.checked_sub(1)?;
        let start = DomStatement::assignment_start(&self.tokens, operator);
        let typed = (start..operator)
            .find(|before| self.text(*before) == ":")
            .and_then(|colon| colon.checked_sub(1));
        Some(typed.unwrap_or(previous))
    }

    fn plain_assignment(tokens: &Tokens<'_>, at: usize) -> bool {
        let text = |at: usize| tokens.items.get(at).map_or("", |token| token.text);
        [
            text(at) == "=",
            !["=", ">"].contains(&text(at + 1)),
            !["=", "!", "<", ">"].contains(&at.checked_sub(1).map_or("", text)),
        ]
        .into_iter()
        .all(|holds| holds)
    }

    fn sink(&self, at: usize) -> Option<Range<usize>> {
        self.member_sink(at).or_else(|| self.function_sink(at))
    }

    fn function_sink(&self, at: usize) -> Option<Range<usize>> {
        (self.text(at + 1) == "(").then_some(())?;
        let kind = self.bindings.kind(self.text(at));
        let reader = DomExpression::new(&self.tokens.items, &self.bindings);
        Self::alias_call(&reader, kind, at + 1)
            .then(|| self.tokens.items[at].start..self.tokens.items[at].end)
    }

    fn member_sink(&self, at: usize) -> Option<Range<usize>> {
        let reader = DomExpression::new(&self.tokens.items, &self.bindings);
        let (property, after) = reader.property(at)?;
        let receiver = reader.receiver(at);
        let call = self.text(after) == "(";
        let reference = reader.member_reference(at);
        let active = Self::active_alias(reference, call, property);
        let alias = Self::alias_call(&reader, active, after);
        [alias, self.forbidden(&reader, receiver, property, after)]
            .contains(&true)
            .then(|| Self::range(&self.tokens, at, after))
    }

    fn active_alias(reference: DomKind, call: bool, property: &str) -> DomKind {
        if [call, property != "bind"].contains(&false) {
            return DomKind::Unknown;
        }
        reference
    }

    fn alias_call(reader: &DomExpression<'_, '_>, kind: DomKind, after: usize) -> bool {
        let sink = [DomKind::MarkupCall, DomKind::StyleCall].contains(&kind);
        let insertion = Self::alias_insertion(reader, kind, after);
        [sink, insertion].contains(&true)
    }

    fn alias_insertion(reader: &DomExpression<'_, '_>, kind: DomKind, after: usize) -> bool {
        if kind == DomKind::NodeInsertion {
            return reader.arguments_create_nodes(after);
        }
        false
    }

    fn range(tokens: &Tokens<'_>, at: usize, after: usize) -> Range<usize> {
        tokens.items[at].start..tokens.items[after - 1].end
    }

    fn forbidden(
        &self,
        reader: &DomExpression<'_, '_>,
        receiver: DomKind,
        property: &str,
        after: usize,
    ) -> bool {
        let write = Self::writes(&self.tokens, after);
        let call = self
            .tokens
            .items
            .get(after)
            .is_some_and(|token| token.text == "(");
        let called_receiver = Self::called_receiver(receiver, call);
        [
            Self::markup_write(receiver, property, write),
            Self::style_write(receiver, property, write),
            Self::markup_call(receiver, property, call),
            Self::style_call(reader, called_receiver, property, after),
            Self::inserts_created(reader, called_receiver, property, after),
        ]
        .contains(&true)
    }

    fn called_receiver(receiver: DomKind, call: bool) -> DomKind {
        if !call {
            return DomKind::Data;
        }
        receiver
    }

    fn writes(tokens: &Tokens<'_>, after: usize) -> bool {
        let at = Self::assignment_at(tokens, after);
        Self::plain_assignment(tokens, at)
    }

    fn assignment_at(tokens: &Tokens<'_>, after: usize) -> usize {
        (after..tokens.items.len())
            .find(|at| !Self::ASSIGNMENT_PREFIX.contains(&tokens.items[*at].text))
            .unwrap_or(after)
    }

    fn markup_write(receiver: DomKind, property: &str, write: bool) -> bool {
        [
            receiver != DomKind::Data,
            Self::HTML_WRITES.contains(&property),
            write,
        ]
        .into_iter()
        .all(|holds| holds)
    }

    fn style_write(receiver: DomKind, property: &str, write: bool) -> bool {
        let style = [
            receiver == DomKind::Style,
            Self::STYLE_WRITES.contains(&property),
        ]
        .contains(&true);
        [receiver != DomKind::Data, style, write]
            .into_iter()
            .all(|holds| holds)
    }

    fn markup_call(receiver: DomKind, property: &str, call: bool) -> bool {
        let document_write = [
            receiver == DomKind::Document,
            ["write", "writeln"].contains(&property),
        ]
        .into_iter()
        .all(|holds| holds);
        let html = [
            receiver != DomKind::Data,
            Self::HTML_CALLS.contains(&property),
        ]
        .into_iter()
        .all(|holds| holds);
        [call, [document_write, html].contains(&true)]
            .into_iter()
            .all(|holds| holds)
    }

    fn style_call(
        reader: &DomExpression<'_, '_>,
        receiver: DomKind,
        property: &str,
        after: usize,
    ) -> bool {
        let rule = [
            receiver != DomKind::Data,
            [
                "insertRule",
                "deleteRule",
                "replaceSync",
                "addRule",
                "removeRule",
            ]
            .contains(&property),
        ]
        .into_iter()
        .all(|holds| holds);
        let replace = [receiver == DomKind::Sheet, property == "replace"]
            .into_iter()
            .all(|holds| holds);
        let sheet = [rule, replace].contains(&true);
        let style = [
            receiver == DomKind::Style,
            ["setProperty", "removeProperty"].contains(&property),
            !reader
                .literal_argument(after)
                .is_some_and(|name| name.starts_with("--")),
        ]
        .into_iter()
        .all(|holds| holds);
        let attribute = [
            receiver != DomKind::Data,
            property == "setAttribute",
            reader.literal_argument(after) == Some("style"),
        ]
        .into_iter()
        .all(|holds| holds);
        [sheet, style, attribute].contains(&true)
    }

    fn inserts_created(
        reader: &DomExpression<'_, '_>,
        receiver: DomKind,
        property: &str,
        after: usize,
    ) -> bool {
        let insertion = [
            receiver != DomKind::Data,
            Self::INSERTIONS.contains(&property),
        ]
        .into_iter()
        .all(|holds| holds);
        if insertion {
            return reader.arguments_create_nodes(after);
        }
        false
    }
}
