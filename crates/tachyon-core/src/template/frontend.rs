use super::braced_attribute::BracedAttribute;
use super::expression::Expression;
use super::model::{
    Assignment, AttributeValue, ConditionalBranch, EventArgument, EventBinding, HydrationPolicy,
    SourceRange, TemplateAttribute, TemplateNode, TemplateNodeKind, TemplateProgram, TextPart,
    is_trivia,
};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::html::{is_event_handler_attribute, is_standard_html_tag, valid_custom_element};
use html5gum::{DefaultEmitter, HtmlString, Span, Spanned, StartTag, Token, Tokenizer};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TemplateFrontend;

impl TemplateFrontend {
    const MAX_HTML_BYTES: u64 = 1_048_576;
    const MAX_EVENT_BINDING_BYTES: usize = 256;
    const MAX_EVENT_PATH_SEGMENTS: usize = 4;
    const MAX_EVENT_ARGUMENTS: usize = 8;

    pub(crate) fn compile_file(
        path: &Path,
        source_path: &str,
        components: &BTreeSet<String>,
    ) -> Result<TemplateProgram, Failure> {
        let source = Self::read_source(path, source_path)?;
        Self::compile(&source, source_path, components)
    }

    pub(crate) fn compile(
        source: &str,
        source_path: &str,
        components: &BTreeSet<String>,
    ) -> Result<TemplateProgram, Failure> {
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
        if source.trim().is_empty() {
            return Err(Failure::one(template_diagnostic(
                1301,
                source_path,
                0,
                source.len(),
                "Template source is empty.",
                "Add at least one semantic HTML element.",
            )));
        }
        if let Some(index) = source.find('\0') {
            return Err(Failure::one(template_diagnostic(
                1301,
                source_path,
                index,
                index + 1,
                "Template source contains a NUL byte.",
                "Remove the NUL byte and save the source as UTF-8.",
            )));
        }
        let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
        Parser::new(&normalized, source_path, components).parse()
    }

    fn read_source(path: &Path, source_path: &str) -> Result<String, Failure> {
        let file = File::open(path).map_err(|error| read_failure(source_path, &error))?;
        let mut bytes = Vec::new();
        file.take(Self::MAX_HTML_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| read_failure(source_path, &error))?;
        if bytes.len() as u64 > Self::MAX_HTML_BYTES {
            return Err(Failure::one(template_diagnostic(
                1301,
                source_path,
                0,
                bytes.len(),
                "Template source exceeds the 1 MiB limit.",
                "Reduce the template below 1 MiB.",
            )));
        }
        String::from_utf8(bytes).map_err(|error| {
            let start = error.utf8_error().valid_up_to();
            Failure::one(template_diagnostic(
                1301,
                source_path,
                start,
                start.saturating_add(1),
                "Template source is not valid UTF-8.",
                "Save Tachyon HTML sources as UTF-8.",
            ))
        })
    }
}

fn read_failure(source_path: &str, error: &std::io::Error) -> Failure {
    Failure::one(template_diagnostic(
        1301,
        source_path,
        0,
        0,
        &format!("Cannot read template source: {error}"),
        "Check the source file permissions.",
    ))
}

struct Parser<'a> {
    source: &'a str,
    source_path: &'a str,
    components: &'a BTreeSet<String>,
    roots: Vec<TemplateNode>,
    stack: Vec<OpenNode>,
    diagnostics: Vec<Diagnostic>,
    cursor: usize,
}

impl<'a> Parser<'a> {
    const MAX_TEMPLATE_DEPTH: usize = 64;
    const MAX_DIAGNOSTICS: usize = 100;
    const VOID_TAGS: &'static [&'static str] = &[
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
        "track", "wbr",
    ];
    const DOCUMENT_CODE: u16 = 1307;
    const DOCUMENT_TAGS: &'static [&'static str] = &["html", "head", "body"];
    const DOCUMENT_HELP: &'static str = "Remove the doctype, <html>, <head> and <body>. Tachyon writes the document around every view. Put the page title and other page metadata in client/manifest.json, under tac.metadata.";

    fn new(source: &'a str, source_path: &'a str, components: &'a BTreeSet<String>) -> Self {
        Self {
            source,
            source_path,
            components,
            roots: Vec::new(),
            stack: Vec::new(),
            diagnostics: Vec::new(),
            cursor: 0,
        }
    }

    fn parse(mut self) -> Result<TemplateProgram, Failure> {
        let emitter = DefaultEmitter::<usize>::new_with_span();
        for result in Tokenizer::new_with_emitter(self.source, emitter) {
            let token = match result {
                Ok(token) => token,
                Err(infallible) => match infallible {},
            };
            self.read_token(token);
        }
        self.close_open_elements();
        self.finish()
    }

    fn read_token(&mut self, token: Token<usize>) {
        if let Some(end) = Self::token_end(&token) {
            self.cursor = end;
        }
        match token {
            Token::StartTag(tag) => self.start_tag(tag),
            Token::EndTag(tag) => {
                let name = String::from_utf8_lossy(&tag.name);
                self.end_tag(&name, span_range(tag.span));
            }
            Token::String(text) => self.text(span_range(text.span)),
            Token::Comment(comment) => self.comment(span_range(comment.span)),
            Token::Doctype(_) => self.doctype(),
            Token::Error(error) => {
                let diagnostic = self.diagnostic(
                    1301,
                    span_range(error.span),
                    "Template contains an HTML tokenizer error.",
                    "Correct the malformed token before building the view.",
                );
                self.push_diagnostic(diagnostic);
            }
        }
    }

    fn token_end(token: &Token<usize>) -> Option<usize> {
        match token {
            Token::StartTag(tag) => Some(tag.span.end),
            Token::EndTag(tag) => Some(tag.span.end),
            Token::String(text) => Some(text.span.end),
            Token::Comment(comment) => Some(comment.span.end),
            Token::Error(error) => Some(error.span.end),
            Token::Doctype(_) => None,
        }
    }

    fn doctype(&mut self) {
        let rest = &self.source[self.cursor..];
        let start = self.cursor + rest.find("<!").unwrap_or_default();
        let length = self.source[start..]
            .find('>')
            .map_or(0, |offset| offset + 1);
        let range = SourceRange {
            start,
            end: start + length,
        };
        self.cursor = range.end;
        let diagnostic = self.diagnostic(
            Self::DOCUMENT_CODE,
            range,
            "A view cannot contain a doctype, because Tachyon writes the document.",
            Self::DOCUMENT_HELP,
        );
        self.push_diagnostic(diagnostic);
    }

    fn text(&mut self, range: SourceRange) {
        let raw = self.source.get(range.start..range.end).unwrap_or_default();
        let inside_style = self.stack.last().is_some_and(|open| open.tag == "style");
        let parts = if inside_style {
            Ok(vec![TextPart::Literal(String::from(raw), range)])
        } else {
            text_parts(raw, self.source_path, range.start)
        };
        match parts {
            Ok(parts) => self.push_node(TemplateNodeKind::Text(parts), range),
            Err(failure) => self.extend_failure(&failure),
        }
    }

    fn comment(&mut self, range: SourceRange) {
        let raw = self.source.get(range.start..range.end).unwrap_or_default();
        let kind = TemplateNodeKind::Comment(String::from(raw));
        self.push_node(kind, range);
    }

    fn close_open_elements(&mut self) {
        while let Some(open) = self.stack.pop() {
            let diagnostic = self.diagnostic(
                1301,
                open.range,
                &format!("Template element '<{}>' is not closed.", open.tag),
                "Close every non-void template element explicitly.",
            );
            self.push_diagnostic(diagnostic);
            self.close(open);
        }
    }

    fn finish(mut self) -> Result<TemplateProgram, Failure> {
        validate_conditionals(&self.roots, &mut self.diagnostics);
        sort_diagnostics(&mut self.diagnostics);
        if !self.diagnostics.is_empty() {
            return Err(Failure::new(self.diagnostics));
        }
        let mut nodes = self.roots;
        desugar_switches(&mut nodes, self.source_path)?;
        Ok(TemplateProgram {
            source_path: String::from(self.source_path),
            nodes,
        })
    }

    fn start_tag(&mut self, tag: StartTag<usize>) {
        let name = String::from_utf8_lossy(&tag.name).into_owned();
        let range = span_range(tag.span);
        if Self::DOCUMENT_TAGS.contains(&name.as_str()) {
            let diagnostic = self.diagnostic(
                Self::DOCUMENT_CODE,
                range,
                &format!("A view cannot contain <{name}>, because Tachyon writes the document."),
                Self::DOCUMENT_HELP,
            );
            self.push_diagnostic(diagnostic);
        }
        let closed = tag.self_closing || Self::VOID_TAGS.contains(&name.as_str());
        let kind = self.start_tag_kind(&name, tag.attributes, range, closed);
        if closed {
            self.push_node(kind, range);
            return;
        }
        if self.stack.len() >= Self::MAX_TEMPLATE_DEPTH {
            let diagnostic = self.diagnostic(
                1301,
                range,
                "Template nesting exceeds the limit of 64.",
                "Flatten the view or component structure.",
            );
            self.push_diagnostic(diagnostic);
            return;
        }
        self.stack.push(OpenNode {
            tag: name,
            kind,
            range,
        });
    }

    fn start_tag_kind(
        &mut self,
        name: &str,
        attributes: BTreeMap<HtmlString, Spanned<HtmlString, usize>>,
        range: SourceRange,
        closed: bool,
    ) -> TemplateNodeKind {
        let attributes = match self.attributes(name, attributes, range) {
            Ok(attributes) => attributes,
            Err(failure) => {
                self.extend_failure(&failure);
                BTreeMap::new()
            }
        };
        match self.node_kind(name, attributes, range) {
            Ok(kind) => kind,
            Err(failure) => {
                self.extend_failure(&failure);
                TemplateNodeKind::Element {
                    tag: String::from(name),
                    attributes: BTreeMap::new(),
                    children: Vec::new(),
                    void: closed,
                }
            }
        }
    }

    fn end_tag(&mut self, name: &str, range: SourceRange) {
        let Some(position) = self.stack.iter().rposition(|open| open.tag == name) else {
            let diagnostic = self.diagnostic(
                1301,
                range,
                &format!("Closing tag '</{name}>' has no matching open element."),
                "Balance template element start and end tags.",
            );
            self.push_diagnostic(diagnostic);
            return;
        };
        while self.stack.len() > position + 1 {
            let open = self.stack.pop().unwrap_or_else(|| unreachable!());
            self.push_diagnostic(template_diagnostic(
                1301,
                self.source_path,
                open.range.start,
                range.end,
                &format!("Element '<{}>' closes out of order.", open.tag),
                "Use explicitly balanced, properly nested template markup.",
            ));
            self.close(open);
        }
        let mut open = self.stack.pop().unwrap_or_else(|| unreachable!());
        open.range.end = range.end;
        self.close(open);
    }

    fn close(&mut self, open: OpenNode) {
        let node = open.finish(self.source_path);
        self.append(node);
    }

    fn attributes(
        &self,
        tag: &str,
        attributes: BTreeMap<HtmlString, Spanned<HtmlString, usize>>,
        tag_range: SourceRange,
    ) -> Result<BTreeMap<String, TemplateAttribute>, Failure> {
        let mut parsed = BTreeMap::new();
        let mut diagnostics = Vec::new();
        for (name, value) in attributes {
            let name = String::from_utf8_lossy(&name).into_owned();
            let range = SourceRange {
                start: value.span.start.max(tag_range.start),
                end: value.span.end.max(tag_range.start),
            };
            let value = String::from_utf8_lossy(&value.value).into_owned();
            match self.attribute(tag, &name, value, range) {
                Ok((name, attribute)) => {
                    parsed.insert(name, attribute);
                }
                Err(failure) => diagnostics.extend_from_slice(failure.diagnostics()),
            }
        }
        if diagnostics.is_empty() {
            Ok(parsed)
        } else {
            Err(Failure::new(diagnostics))
        }
    }

    fn attribute(
        &self,
        tag: &str,
        name: &str,
        value: String,
        range: SourceRange,
    ) -> Result<(String, TemplateAttribute), Failure> {
        if let Some(event) = name.strip_prefix("on:") {
            return self.event_attribute(event, &value, range);
        }
        if is_event_handler_attribute(name) {
            return Err(self.failure(
                1306,
                range,
                "Native event attributes are outside the view contract.",
                "Use on:<event> to bind an exported handler instead.",
            ));
        }
        if name == ":hydrate" {
            return Err(self.failure(
                1404,
                range,
                "Island hydration policy must be a static literal.",
                "Use hydrate=\"load|idle|visible|interaction|never\".",
            ));
        }
        let Some(dynamic_name) = name.strip_prefix(':') else {
            let inside_style = self.stack.iter().any(|open| open.tag == "style");
            if let Some(braced) = BracedAttribute::find(name, &value).filter(|_| !inside_style) {
                return Err(self.failure(
                    BracedAttribute::CODE,
                    range,
                    &braced.message(),
                    &braced.help(),
                ));
            }
            let attribute = TemplateAttribute {
                value: AttributeValue::Static(value),
                range,
            };
            return Ok((String::from(name), attribute));
        };
        self.dynamic_attribute(tag, dynamic_name, value, range)
    }

    fn event_attribute(
        &self,
        event: &str,
        value: &str,
        range: SourceRange,
    ) -> Result<(String, TemplateAttribute), Failure> {
        match event_binding(event, value, self.source_path, range) {
            Ok((marker, binding)) => {
                let attribute = TemplateAttribute {
                    value: AttributeValue::Event(binding),
                    range,
                };
                Ok((marker, attribute))
            }
            Err(message) => Err(self.failure(
                1306,
                range,
                message,
                "Bind an exported handler, as in on:click=\"increment()\".",
            )),
        }
    }

    fn dynamic_attribute(
        &self,
        tag: &str,
        dynamic_name: &str,
        value: String,
        range: SourceRange,
    ) -> Result<(String, TemplateAttribute), Failure> {
        if dynamic_name.is_empty() || tag == "logic" && dynamic_name == "else" {
            return Err(self.failure(
                1304,
                range,
                "Dynamic attribute name is invalid.",
                "Place ':' before a valid HTML attribute or component property name.",
            ));
        }
        let is_control = matches!((tag, dynamic_name), ("loop", "for") | ("for", "each"));
        let value = if is_control {
            AttributeValue::Control(value)
        } else {
            AttributeValue::Dynamic(Expression::parse(&value, self.source_path, range.start)?)
        };
        Ok((
            String::from(dynamic_name),
            TemplateAttribute { value, range },
        ))
    }

    fn node_kind(
        &self,
        name: &str,
        mut attributes: BTreeMap<String, TemplateAttribute>,
        range: SourceRange,
    ) -> Result<TemplateNodeKind, Failure> {
        let source_path = self.source_path;
        match name {
            "logic" => conditional_logic(&mut attributes, source_path, range),
            "if" => direct_if(&mut attributes, source_path, range),
            "else" => direct_else(&attributes, source_path, range),
            "switch" => switch_value(&mut attributes, source_path, range),
            "case" => switch_case(&mut attributes, source_path, range),
            "loop" => iteration(&mut attributes, "for", source_path, range),
            "for" => iteration(&mut attributes, "each", source_path, range),
            "script" => Err(self.failure(
                1306,
                range,
                "Executable script elements are outside the Phase 3 view contract.",
                "Use a colocated Tac island companion.",
            )),
            "slot" => slot_kind(&mut attributes, source_path, range),
            _ if self.components.contains(name) => {
                component_kind(name, attributes, source_path, range)
            }
            _ if is_standard_html_tag(name) || valid_custom_element(name) => {
                self.element_kind(name, attributes, range)
            }
            _ => Err(self.failure(
                1402,
                range,
                &format!("Unknown Tac component '<{name}>'."),
                "Register a custom-element-shaped component or use standard HTML.",
            )),
        }
    }

    fn element_kind(
        &self,
        name: &str,
        attributes: BTreeMap<String, TemplateAttribute>,
        range: SourceRange,
    ) -> Result<TemplateNodeKind, Failure> {
        if attributes.contains_key("hydrate") {
            return Err(self.failure(
                1404,
                range,
                "Only a registered Tac component may declare hydrate.",
                "Remove hydrate or register the component template.",
            ));
        }
        Ok(TemplateNodeKind::Element {
            tag: String::from(name),
            attributes,
            children: Vec::new(),
            void: Self::VOID_TAGS.contains(&name),
        })
    }

    fn push_node(&mut self, kind: TemplateNodeKind, range: SourceRange) {
        self.append(TemplateNode {
            kind,
            source_path: String::from(self.source_path),
            range,
        });
    }

    fn append(&mut self, node: TemplateNode) {
        let range = node.range;
        let Some(parent) = self.stack.last_mut() else {
            self.roots.push(node);
            return;
        };
        if let Some(children) = parent.kind.children_mut() {
            children.push(node);
            return;
        }
        let message = format!("Element '<{}>' cannot contain child content.", parent.tag);
        let diagnostic = self.diagnostic(
            1301,
            range,
            &message,
            "Leave the element empty and close it immediately.",
        );
        self.push_diagnostic(diagnostic);
    }

    fn extend_failure(&mut self, failure: &Failure) {
        for value in failure.diagnostics() {
            self.push_diagnostic(value.clone());
        }
    }

    fn push_diagnostic(&mut self, value: Diagnostic) {
        if self.diagnostics.len() < Self::MAX_DIAGNOSTICS {
            self.diagnostics.push(value);
        }
    }

    fn diagnostic(&self, number: u16, range: SourceRange, message: &str, help: &str) -> Diagnostic {
        template_diagnostic(
            number,
            self.source_path,
            range.start,
            range.end,
            message,
            help,
        )
    }

    fn failure(&self, number: u16, range: SourceRange, message: &str, help: &str) -> Failure {
        Failure::one(self.diagnostic(number, range, message, help))
    }
}

struct OpenNode {
    tag: String,
    kind: TemplateNodeKind,
    range: SourceRange,
}

impl OpenNode {
    fn finish(self, source_path: &str) -> TemplateNode {
        TemplateNode {
            kind: self.kind,
            source_path: String::from(source_path),
            range: self.range,
        }
    }
}

fn span_range(span: Span<usize>) -> SourceRange {
    SourceRange {
        start: span.start,
        end: span.end,
    }
}

fn component_kind(
    name: &str,
    mut attributes: BTreeMap<String, TemplateAttribute>,
    source_path: &str,
    range: SourceRange,
) -> Result<TemplateNodeKind, Failure> {
    let hydrate = take_hydrate(&mut attributes, source_path, range)?;
    Ok(TemplateNodeKind::Component {
        name: String::from(name),
        properties: attributes,
        hydrate,
        children: Vec::new(),
    })
}

fn switch_value(
    attributes: &mut BTreeMap<String, TemplateAttribute>,
    source_path: &str,
    range: SourceRange,
) -> Result<TemplateNodeKind, Failure> {
    let Some(value) = take_dynamic(attributes, "value") else {
        return Err(control_failure(
            source_path,
            range,
            "<switch> requires :value=\"expression\".",
        ));
    };
    reject_attributes(attributes, source_path, range, "switch")?;
    Ok(TemplateNodeKind::Switch {
        value,
        children: Vec::new(),
    })
}

fn slot_kind(
    attributes: &mut BTreeMap<String, TemplateAttribute>,
    source_path: &str,
    range: SourceRange,
) -> Result<TemplateNodeKind, Failure> {
    let name = attributes.remove("name");
    reject_attributes(attributes, source_path, range, "slot")?;
    let Some(attribute) = name else {
        return Ok(TemplateNodeKind::Slot(None));
    };
    match attribute.value {
        AttributeValue::Static(name) if !name.trim().is_empty() => {
            Ok(TemplateNodeKind::Slot(Some(name)))
        }
        _ => Err(control_failure(
            source_path,
            range,
            "<slot name> needs a fixed name that is not empty, such as <slot name=\"title\">.",
        )),
    }
}

fn switch_case(
    attributes: &mut BTreeMap<String, TemplateAttribute>,
    source_path: &str,
    range: SourceRange,
) -> Result<TemplateNodeKind, Failure> {
    if let Some(when) = take_dynamic(attributes, "when") {
        reject_attributes(attributes, source_path, range, "case")?;
        return Ok(TemplateNodeKind::Case {
            when: Some(when),
            children: Vec::new(),
        });
    }
    if take_static_flag(attributes, "default") {
        reject_attributes(attributes, source_path, range, "case")?;
        return Ok(TemplateNodeKind::Case {
            when: None,
            children: Vec::new(),
        });
    }
    Err(control_failure(
        source_path,
        range,
        "<case> requires exactly one of :when or default.",
    ))
}

fn desugar_switches(nodes: &mut Vec<TemplateNode>, source_path: &str) -> Result<(), Failure> {
    let mut index = 0;
    while index < nodes.len() {
        if let TemplateNodeKind::Switch { .. } = nodes[index].kind {
            let node = nodes.remove(index);
            let branches = expand_switch(node, source_path)?;
            let count = branches.len();
            for (offset, branch) in branches.into_iter().enumerate() {
                nodes.insert(index + offset, branch);
            }
            index += count;
            continue;
        }
        if let Some(children) = nodes[index].kind.children_mut() {
            desugar_switches(children, source_path)?;
        }
        index += 1;
    }
    Ok(())
}

fn expand_switch(node: TemplateNode, source_path: &str) -> Result<Vec<TemplateNode>, Failure> {
    let TemplateNodeKind::Switch { value, children } = node.kind else {
        unreachable!("caller matched a switch")
    };
    let mut branches = Vec::new();
    let mut seen_default = false;
    for child in children {
        let is_case = matches!(child.kind, TemplateNodeKind::Case { .. });
        if !is_case && is_trivia(&child) {
            continue;
        }
        if is_case && seen_default {
            return Err(control_failure(
                source_path,
                child.range,
                "<case default> must be the last case.",
            ));
        }
        let is_first = branches.is_empty();
        let branch = switch_branch(child, &value, is_first, &node.source_path, source_path)?;
        if matches!(
            branch.kind,
            TemplateNodeKind::Conditional {
                branch: ConditionalBranch::Else,
                ..
            }
        ) {
            seen_default = true;
        }
        branches.push(branch);
    }
    if branches.is_empty() {
        return Err(control_failure(
            source_path,
            node.range,
            "<switch> requires at least one <case>.",
        ));
    }
    Ok(branches)
}

fn switch_branch(
    child: TemplateNode,
    value: &Expression,
    is_first: bool,
    switch_source_path: &str,
    source_path: &str,
) -> Result<TemplateNode, Failure> {
    let TemplateNodeKind::Case { when, mut children } = child.kind else {
        return Err(control_failure(
            source_path,
            child.range,
            "<switch> may only contain <case> children.",
        ));
    };
    desugar_switches(&mut children, source_path)?;
    let (condition, branch) = match when {
        None if is_first => {
            return Err(control_failure(
                source_path,
                child.range,
                "<switch> needs at least one <case :when> before <case default>.",
            ));
        }
        None => (None, ConditionalBranch::Else),
        Some(when) if is_first => (Some(value.equals(&when)), ConditionalBranch::If),
        Some(when) => (Some(value.equals(&when)), ConditionalBranch::ElseIf),
    };
    Ok(TemplateNode {
        kind: TemplateNodeKind::Conditional {
            condition,
            branch,
            children,
        },
        range: child.range,
        source_path: String::from(switch_source_path),
    })
}

fn conditional_logic(
    attributes: &mut BTreeMap<String, TemplateAttribute>,
    source_path: &str,
    range: SourceRange,
) -> Result<TemplateNodeKind, Failure> {
    if let Some(condition) = take_dynamic(attributes, "if") {
        return logic_branch(
            attributes,
            Some(condition),
            ConditionalBranch::If,
            source_path,
            range,
        );
    }
    if let Some(condition) = take_dynamic(attributes, "else-if") {
        return logic_branch(
            attributes,
            Some(condition),
            ConditionalBranch::ElseIf,
            source_path,
            range,
        );
    }
    if take_static_flag(attributes, "else") {
        return logic_branch(
            attributes,
            None,
            ConditionalBranch::Else,
            source_path,
            range,
        );
    }
    Err(control_failure(
        source_path,
        range,
        "<logic> requires exactly one of :if, :else-if, or else.",
    ))
}

fn logic_branch(
    attributes: &BTreeMap<String, TemplateAttribute>,
    condition: Option<Expression>,
    branch: ConditionalBranch,
    source_path: &str,
    range: SourceRange,
) -> Result<TemplateNodeKind, Failure> {
    reject_attributes(attributes, source_path, range, "logic")?;
    Ok(TemplateNodeKind::Conditional {
        condition,
        branch,
        children: Vec::new(),
    })
}

fn direct_if(
    attributes: &mut BTreeMap<String, TemplateAttribute>,
    source_path: &str,
    range: SourceRange,
) -> Result<TemplateNodeKind, Failure> {
    let Some(condition) = take_dynamic(attributes, "when") else {
        return Err(Failure::one(template_diagnostic(
            1103,
            source_path,
            range.start,
            range.end,
            "<if> requires the Phase 3 :when=\"expression\" contract.",
            "Replace legacy condition attributes with :when.",
        )));
    };
    reject_attributes(attributes, source_path, range, "if")?;
    Ok(TemplateNodeKind::Conditional {
        condition: Some(condition),
        branch: ConditionalBranch::If,
        children: Vec::new(),
    })
}

fn direct_else(
    attributes: &BTreeMap<String, TemplateAttribute>,
    source_path: &str,
    range: SourceRange,
) -> Result<TemplateNodeKind, Failure> {
    reject_attributes(attributes, source_path, range, "else")?;
    Ok(TemplateNodeKind::Conditional {
        condition: None,
        branch: ConditionalBranch::Else,
        children: Vec::new(),
    })
}

fn iteration(
    attributes: &mut BTreeMap<String, TemplateAttribute>,
    attribute_name: &str,
    source_path: &str,
    range: SourceRange,
) -> Result<TemplateNodeKind, Failure> {
    let Some(attribute) = attributes.remove(attribute_name) else {
        let element = if attribute_name == "for" {
            "loop"
        } else {
            "for"
        };
        return Err(control_failure(
            source_path,
            range,
            &format!("<{element}> requires :{attribute_name}=\"item of values\"."),
        ));
    };
    let AttributeValue::Control(declaration) = attribute.value else {
        return Err(control_failure(
            source_path,
            range,
            "Iteration declaration must use a dynamic control attribute.",
        ));
    };
    if declaration.contains(';') {
        return counted_iteration(
            &declaration,
            attribute.range.start,
            source_path,
            range,
            attributes,
        );
    }
    let separator = if attribute_name == "for" {
        " of "
    } else {
        " in "
    };
    list_iteration(
        &declaration,
        separator,
        attribute.range.start,
        source_path,
        range,
        attributes,
    )
}

fn list_iteration(
    declaration: &str,
    separator: &str,
    start: usize,
    source_path: &str,
    range: SourceRange,
    attributes: &BTreeMap<String, TemplateAttribute>,
) -> Result<TemplateNodeKind, Failure> {
    let Some((binding, iterable)) = declaration.split_once(separator) else {
        return Err(control_failure(
            source_path,
            range,
            "Iteration must use 'binding of expression' or 'binding in expression'.",
        ));
    };
    let binding = without_declaration_keyword(binding);
    if !valid_binding(binding) {
        return Err(control_failure(
            source_path,
            range,
            "Iteration binding is not a portable identifier.",
        ));
    }
    let iterable_offset = declaration.find(iterable).unwrap_or_default();
    let iterable_start = start.saturating_add(iterable_offset);
    let iterable = Expression::parse(iterable.trim(), source_path, iterable_start)?;
    reject_attributes(attributes, source_path, range, "iteration")?;
    Ok(TemplateNodeKind::Iteration {
        binding: String::from(binding),
        iterable,
        children: Vec::new(),
    })
}

fn without_declaration_keyword(binding: &str) -> &str {
    let binding = binding.trim();
    if let Some(rest) = binding.strip_prefix("const ") {
        return rest.trim();
    }
    if let Some(rest) = binding.strip_prefix("let ") {
        return rest.trim();
    }
    binding
}

fn counted_iteration(
    declaration: &str,
    start: usize,
    source_path: &str,
    range: SourceRange,
    attributes: &BTreeMap<String, TemplateAttribute>,
) -> Result<TemplateNodeKind, Failure> {
    let clauses: Vec<_> = declaration.split(';').map(str::trim).collect();
    let [init, test, update] = clauses.as_slice() else {
        return Err(control_failure(
            source_path,
            range,
            "A counted loop is 'let i = start; i < limit; i++'.",
        ));
    };
    let (binding, from) = counted_initialization(init, source_path, range)?;
    let (comparison, to) = counted_comparison(test, binding, source_path, range)?;
    let step = counted_step(update, binding, comparison, source_path, range)?;
    reject_attributes(attributes, source_path, range, "iteration")?;
    Ok(TemplateNodeKind::CountedIteration {
        binding: String::from(binding),
        from: Expression::parse(from, source_path, start)?,
        comparison,
        to: Expression::parse(to, source_path, start)?,
        step: Expression::parse(step, source_path, start)?,
        children: Vec::new(),
    })
}

fn counted_initialization<'a>(
    init: &'a str,
    source_path: &str,
    range: SourceRange,
) -> Result<(&'a str, &'a str), Failure> {
    let Some(("let", rest)) = init.split_once(char::is_whitespace) else {
        return Err(control_failure(
            source_path,
            range,
            "A counted loop must declare its counter with 'let'.",
        ));
    };
    let Some((binding, from)) = rest.split_once('=') else {
        return Err(control_failure(
            source_path,
            range,
            "A counted loop needs 'let i = start'.",
        ));
    };
    let binding = binding.trim();
    if !valid_binding(binding) {
        return Err(control_failure(
            source_path,
            range,
            "Counted loop counter is not a portable identifier.",
        ));
    }
    Ok((binding, from.trim()))
}

fn counted_comparison<'a>(
    test: &'a str,
    binding: &str,
    source_path: &str,
    range: SourceRange,
) -> Result<(tachyon_contracts::CountedComparison, &'a str), Failure> {
    use tachyon_contracts::CountedComparison as Comparison;
    for (operator, comparison) in [
        ("<=", Comparison::Le),
        (">=", Comparison::Ge),
        ("<", Comparison::Lt),
        (">", Comparison::Gt),
    ] {
        let Some((subject, limit)) = test.split_once(operator) else {
            continue;
        };
        if subject.trim() != binding {
            return Err(control_failure(
                source_path,
                range,
                "A counted loop must test its own counter.",
            ));
        }
        return Ok((comparison, limit.trim()));
    }
    Err(control_failure(
        source_path,
        range,
        "A counted loop tests its counter with <, <=, > or >=.",
    ))
}

fn counted_step<'a>(
    update: &'a str,
    binding: &str,
    comparison: tachyon_contracts::CountedComparison,
    source_path: &str,
    range: SourceRange,
) -> Result<&'a str, Failure> {
    let Some((counter, magnitude, ascending)) = split_counted_update(update) else {
        return Err(control_failure(
            source_path,
            range,
            "A counted loop advances with i++, i--, i += step or i -= step.",
        ));
    };
    if counter.trim() != binding || ascending != comparison.ascending() {
        return Err(control_failure(
            source_path,
            range,
            "A counted loop must advance its own counter toward its limit.",
        ));
    }
    let magnitude = magnitude.trim();
    if is_invalid_step(magnitude) {
        return Err(control_failure(
            source_path,
            range,
            "A counted loop step must be a positive finite magnitude.",
        ));
    }
    Ok(magnitude)
}

fn is_invalid_step(magnitude: &str) -> bool {
    if magnitude.starts_with('-') {
        return true;
    }
    let Ok(value) = magnitude.parse::<f64>() else {
        return false;
    };
    !value.is_finite() || value <= 0.0
}

fn split_counted_update(update: &str) -> Option<(&str, &str, bool)> {
    for (operator, ascending) in [("++", true), ("--", false)] {
        if let Some(counter) = update.strip_suffix(operator) {
            return Some((counter, "1", ascending));
        }
    }
    for (operator, ascending) in [("+=", true), ("-=", false)] {
        if let Some((counter, magnitude)) = update.split_once(operator) {
            return Some((counter, magnitude, ascending));
        }
    }
    None
}

fn take_hydrate(
    attributes: &mut BTreeMap<String, TemplateAttribute>,
    source_path: &str,
    range: SourceRange,
) -> Result<Option<HydrationPolicy>, Failure> {
    let Some(attribute) = attributes.remove("hydrate") else {
        return Ok(None);
    };
    let AttributeValue::Static(value) = attribute.value else {
        return Err(Failure::one(template_diagnostic(
            1404,
            source_path,
            range.start,
            range.end,
            "Island hydration policy must be a static literal.",
            "Use hydrate=\"load|idle|visible|interaction|never\".",
        )));
    };
    let Some(policy) = HydrationPolicy::parse(&value) else {
        return Err(Failure::one(template_diagnostic(
            1404,
            source_path,
            attribute.range.start,
            attribute.range.end,
            "Island hydration policy is not supported.",
            "Use load, idle, visible, interaction, or never.",
        )));
    };
    Ok(Some(policy))
}

fn take_dynamic(
    attributes: &mut BTreeMap<String, TemplateAttribute>,
    name: &str,
) -> Option<Expression> {
    let attribute = attributes.remove(name)?;
    let AttributeValue::Dynamic(expression) = attribute.value else {
        return None;
    };
    Some(expression)
}

fn take_static_flag(attributes: &mut BTreeMap<String, TemplateAttribute>, name: &str) -> bool {
    let Some(attribute) = attributes.remove(name) else {
        return false;
    };
    matches!(attribute.value, AttributeValue::Static(value) if value.is_empty())
}

fn reject_attributes(
    attributes: &BTreeMap<String, TemplateAttribute>,
    source_path: &str,
    range: SourceRange,
    element: &str,
) -> Result<(), Failure> {
    if attributes.is_empty() {
        return Ok(());
    }
    Err(control_failure(
        source_path,
        range,
        &format!("<{element}> contains unsupported attributes."),
    ))
}

fn validate_conditionals(nodes: &[TemplateNode], diagnostics: &mut Vec<Diagnostic>) {
    let mut after_if_branch = false;
    for node in nodes {
        match &node.kind {
            TemplateNodeKind::Conditional {
                branch: ConditionalBranch::If,
                ..
            } => after_if_branch = true,
            TemplateNodeKind::Conditional {
                branch: ConditionalBranch::ElseIf,
                ..
            } if after_if_branch => {}
            TemplateNodeKind::Conditional {
                branch: ConditionalBranch::Else,
                ..
            } if after_if_branch => after_if_branch = false,
            TemplateNodeKind::Conditional {
                branch: ConditionalBranch::ElseIf | ConditionalBranch::Else,
                ..
            } => diagnostics.push(template_diagnostic(
                1302,
                &node.source_path,
                node.range.start,
                node.range.end,
                "Conditional else branch has no preceding if branch.",
                "Place else-if or else immediately after an if branch.",
            )),
            _ if is_trivia(node) => {}
            _ => after_if_branch = false,
        }
        if let Some(children) = node.kind.children() {
            validate_conditionals(children, diagnostics);
        }
    }
}

fn decode_text_references(value: &str) -> String {
    if !value.contains('&') {
        return String::from(value);
    }
    let escaped = value.replace('<', "&lt;");
    let mut result = String::with_capacity(value.len());
    for token in Tokenizer::new(escaped.as_str()) {
        if let Ok(Token::String(text)) = token {
            result.push_str(&String::from_utf8_lossy(&text.value));
        }
    }
    result
}

fn text_parts(
    value: &str,
    source_path: &str,
    source_start: usize,
) -> Result<Vec<TextPart>, Failure> {
    let bytes = value.as_bytes();
    let mut parts = Vec::new();
    let mut literal_start = 0;
    let mut current = 0;
    while current < value.len() {
        if is_escaped_brace(bytes, current) {
            push_literal(&mut parts, value, source_start, literal_start, current);
            parts.push(escaped_brace(value, source_start, current));
            current += 2;
            literal_start = current;
            continue;
        }
        if bytes[current] != b'{' {
            let character = value[current..]
                .chars()
                .next()
                .unwrap_or_else(|| unreachable!());
            current += character.len_utf8();
            continue;
        }
        push_literal(&mut parts, value, source_start, literal_start, current);
        let (interpolation, next) = interpolation(value, source_path, source_start, current)?;
        parts.push(interpolation);
        current = next;
        literal_start = current;
    }
    push_literal(&mut parts, value, source_start, literal_start, value.len());
    Ok(parts)
}

fn is_escaped_brace(bytes: &[u8], index: usize) -> bool {
    if bytes[index] != b'\\' {
        return false;
    }
    matches!(bytes.get(index + 1), Some(b'{' | b'}'))
}

fn escaped_brace(value: &str, source_start: usize, backslash: usize) -> TextPart {
    let brace = String::from(&value[backslash + 1..backslash + 2]);
    let range = SourceRange {
        start: source_start + backslash,
        end: source_start + backslash + 2,
    };
    TextPart::Literal(brace, range)
}

fn push_literal(
    parts: &mut Vec<TextPart>,
    value: &str,
    source_start: usize,
    start: usize,
    end: usize,
) {
    if start >= end {
        return;
    }
    parts.push(TextPart::Literal(
        decode_text_references(&value[start..end]),
        SourceRange {
            start: source_start + start,
            end: source_start + end,
        },
    ));
}

fn interpolation(
    value: &str,
    source_path: &str,
    source_start: usize,
    open: usize,
) -> Result<(TextPart, usize), Failure> {
    let Some(relative_close) = value[open + 1..].find('}') else {
        return Err(Failure::one(template_diagnostic(
            1303,
            source_path,
            source_start + open,
            source_start + value.len(),
            "Text interpolation is missing '}'.",
            "Close the bounded template expression.",
        )));
    };
    let close = open + 1 + relative_close;
    let inner = &value[open + 1..close];
    let expression_source = inner.trim();
    if expression_source.starts_with('!') {
        return Err(Failure::one(template_diagnostic(
            1306,
            source_path,
            source_start + open,
            source_start + close + 1,
            "Raw HTML interpolation is not supported.",
            "Use escaped interpolation and semantic component structure.",
        )));
    }
    let leading_spaces = inner.len() - inner.trim_start().len();
    let expression_start = source_start + open + 1 + leading_spaces;
    let expression = Expression::parse(expression_source, source_path, expression_start)?;
    let range = SourceRange {
        start: source_start + open,
        end: source_start + close + 1,
    };
    Ok((TextPart::Interpolation(expression, range), close + 1))
}

fn valid_binding(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && !matches!(first, b'_' | b'$') {
        return false;
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$'))
}

fn control_failure(source_path: &str, range: SourceRange, message: &str) -> Failure {
    Failure::one(template_diagnostic(
        1302,
        source_path,
        range.start,
        range.end,
        message,
        "Use the control syntax documented in PHASE_3_SPEC.md.",
    ))
}

fn event_binding(
    event: &str,
    value: &str,
    source_path: &str,
    range: SourceRange,
) -> Result<(String, EventBinding), &'static str> {
    validate_event_name(event)?;
    let value = value.trim();
    if value.is_empty() || value.len() > TemplateFrontend::MAX_EVENT_BINDING_BYTES {
        return Err("Event binding must be 1 to 256 characters.");
    }
    let marker = format!("data-tac-on-{}", event.replace(':', "__"));
    if let Some((target, operator, expression)) = split_assignment(value) {
        let binding = EventBinding {
            handler: String::new(),
            arguments: vec![event_argument(expression, source_path, range)?],
            assign: Some(Assignment { target, operator }),
        };
        return Ok((marker, binding));
    }
    let (name, arguments) = handler_call(value, source_path, range)?;
    if !is_member_path(name) {
        return Err("Event handler must name one handler, as save() or basket.add().");
    }
    let binding = EventBinding {
        handler: String::from(name),
        arguments,
        assign: None,
    };
    Ok((marker, binding))
}

fn validate_event_name(event: &str) -> Result<(), &'static str> {
    if event.is_empty() || event.len() > 64 {
        return Err("Event name must be 1 to 64 characters.");
    }
    let is_allowed = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b':';
    if !event.bytes().all(is_allowed) {
        return Err("Event name must be lowercase alphanumeric, with ':' for namespaces.");
    }
    Ok(())
}

fn handler_call<'a>(
    value: &'a str,
    source_path: &str,
    range: SourceRange,
) -> Result<(&'a str, Vec<EventArgument>), &'static str> {
    let Some((name, rest)) = value.split_once('(') else {
        return Ok((value, Vec::new()));
    };
    let Some(inner) = rest.strip_suffix(')') else {
        return Err("Event binding must close its argument list.");
    };
    let arguments = handler_arguments(inner.trim(), source_path, range)?;
    Ok((name.trim(), arguments))
}

fn handler_arguments(
    inner: &str,
    source_path: &str,
    range: SourceRange,
) -> Result<Vec<EventArgument>, &'static str> {
    let mut arguments = Vec::new();
    if inner.is_empty() {
        return Ok(arguments);
    }
    for argument in inner.split(',') {
        if arguments.len() >= TemplateFrontend::MAX_EVENT_ARGUMENTS {
            return Err("Event binding accepts at most eight arguments.");
        }
        arguments.push(event_argument(argument.trim(), source_path, range)?);
    }
    Ok(arguments)
}

fn split_assignment(value: &str) -> Option<(String, String, &str)> {
    let bytes = value.as_bytes();
    let index = bytes.iter().position(|byte| *byte == b'=')?;
    let next = bytes.get(index + 1);
    let previous = if index == 0 {
        None
    } else {
        bytes.get(index - 1)
    };
    if next == Some(&b'=') || matches!(previous, Some(b'!' | b'<' | b'>' | b'=')) {
        return None;
    }
    let (left, right) = value.split_at(index);
    let right = right.get(1..)?.trim();
    let (target, operator) = split_compound_operator(left.trim());
    if right.is_empty() || !is_member_path(target) {
        return None;
    }
    Some((String::from(target), operator, right))
}

fn split_compound_operator(left: &str) -> (&str, String) {
    for compound in ['+', '-', '*', '/'] {
        if let Some(target) = left.strip_suffix(compound) {
            return (target.trim(), format!("{compound}="));
        }
    }
    (left, String::from("="))
}

fn is_member_path(name: &str) -> bool {
    let Some((service, member)) = name.split_once('.') else {
        return is_handler_identifier(name);
    };
    !service.starts_with('$') && is_handler_identifier(service) && is_handler_identifier(member)
}

fn is_handler_identifier(segment: &str) -> bool {
    is_plain_identifier(segment.trim_start_matches('$'))
}

fn is_plain_identifier(segment: &str) -> bool {
    let Some(first) = segment.bytes().next() else {
        return false;
    };
    if segment.len() > 64 {
        return false;
    }
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }
    segment
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn event_argument(
    value: &str,
    source_path: &str,
    range: SourceRange,
) -> Result<EventArgument, &'static str> {
    if let Some(path) = value.strip_prefix("$event") {
        return event_path(path).map(EventArgument::EventPath);
    }
    if let Ok(literal) = event_literal(value) {
        return Ok(EventArgument::Literal(literal));
    }
    let Ok(expression) = Expression::parse(value, source_path, range.start) else {
        return Err(
            "Event binding arguments must be literals, $event property paths, or bounded \
             template expressions.",
        );
    };
    Ok(EventArgument::Scope(expression))
}

fn event_path(path: &str) -> Result<String, &'static str> {
    if path.is_empty() {
        return Ok(String::new());
    }
    let Some(path) = path.strip_prefix('.') else {
        return Err("Read a property off $event with a dot, as in $event.target.value.");
    };
    let segments: Vec<&str> = path.split('.').collect();
    if segments.len() > TemplateFrontend::MAX_EVENT_PATH_SEGMENTS {
        return Err("$event property path is limited to four segments.");
    }
    for segment in &segments {
        if !is_plain_identifier(segment) {
            return Err("$event property path segments must be plain identifiers.");
        }
    }
    Ok(String::from(path))
}

fn event_literal(value: &str) -> Result<String, &'static str> {
    if value == "true" || value == "false" || value == "null" {
        return Ok(String::from(value));
    }
    if value.parse::<f64>().is_ok_and(f64::is_finite) {
        return Ok(String::from(value));
    }
    let Some(text) = quoted_text(value) else {
        return Err(
            "Event binding arguments must be literals or $event property paths, as in \
             $event.target.value.",
        );
    };
    if text.contains(['\'', '"', '\\', ',', '(', ')']) || text.chars().any(char::is_control) {
        return Err("Event binding string arguments must be simple literals.");
    }
    Ok(format!("\"{text}\""))
}

fn quoted_text(value: &str) -> Option<&str> {
    for quote in ['\'', '"'] {
        let text = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote));
        if text.is_some() {
            return text;
        }
    }
    None
}

fn template_diagnostic(
    number: u16,
    source_path: &str,
    start: usize,
    end: usize,
    message: &str,
    help: &str,
) -> Diagnostic {
    diagnostic(
        number,
        message,
        Some(String::from(help)),
        source_span(source_path, start, end),
    )
}

fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|left, right| {
        let left_position = diagnostic_position(left);
        let right_position = diagnostic_position(right);
        left_position
            .cmp(&right_position)
            .then_with(|| left.code.cmp(&right.code))
    });
}

fn diagnostic_position(value: &Diagnostic) -> Option<(&String, u64, u64)> {
    let span = value.spans.first()?;
    Some((&span.file, span.start, span.end))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{
        EventArgument, EventBinding, SourceRange, TemplateFrontend, TemplateProgram, event_binding,
    };
    use crate::Failure;
    use std::collections::BTreeSet;

    fn compile(source: &str, components: &BTreeSet<String>) -> Result<TemplateProgram, Failure> {
        TemplateFrontend::compile(source, "client/pages/tac.html", components)
    }

    fn compile_without_components(source: &str) -> Result<TemplateProgram, Failure> {
        compile(source, &BTreeSet::new())
    }

    fn bind(event: &str, value: &str) -> Result<(String, EventBinding), &'static str> {
        let range = SourceRange { start: 0, end: 0 };
        event_binding(event, value, "client/pages/tac.html", range)
    }

    fn binding_parts(event: &str, value: &str) -> (String, String, Vec<EventArgument>) {
        let (marker, binding) = bind(event, value).expect(value);
        (marker, binding.handler, binding.arguments)
    }

    fn input_arguments(value: &str) -> Vec<EventArgument> {
        bind("input", value).expect(value).1.arguments
    }

    #[test]
    fn controls_components_and_interpolation_lower_to_view_ir() {
        let components = BTreeSet::from([String::from("product-card")]);
        let program = compile(
            r#"<logic :if="show"><product-card :item="item">{item.name}</product-card></logic><logic else>none</logic>"#,
            &components,
        )
        .unwrap_or_else(|_| unreachable!());
        let json = serde_json::to_value(program.view_ir()).unwrap_or_else(|_| unreachable!());
        assert_eq!(json["root"]["kind"], "conditional");
        let text = serde_json::to_string(&json).unwrap_or_else(|_| unreachable!());
        assert!(text.contains("\"kind\":\"conditional\""));
        assert!(text.contains("\"kind\":\"component\""));
        assert!(!text.contains("\"tag\":\"logic\""));
    }

    #[test]
    fn parser_recovers_multiple_structure_and_expression_failures() {
        let error =
            compile_without_components("<logic else>x</logic><p>{bad syntax}</p><div><span></div>")
                .expect_err("invalid template");
        assert!(error.diagnostics().len() >= 3);
    }

    #[test]
    fn content_after_an_open_slot_fails_closed_instead_of_panicking() {
        for source in [
            "<main><slot>text</slot></main>",
            "<slot><p>child</p></slot>",
            "<main><slot>{ text <pr<slot><p>child</p></slot>\n",
            "main><slot>{\u{d}\u{d}\u{d}\u{d}\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}-\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}\u{d}\u{d}<pr<main><slot>{\u{d}\u{d}\u{d}\u{d}\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}-\u{11}\u{11}\u{11}\u{11}\u{11}\u{11}\u{d}\u{d}<pr\u{d}",
        ] {
            let error = compile_without_components(source).expect_err(source);
            let has_structure_error = error
                .diagnostics()
                .iter()
                .any(|value| value.code.number() == 1301);
            assert!(has_structure_error, "{source}");
        }
    }

    #[test]
    fn malformed_controls_attributes_and_sources_fail_closed() {
        for source in [
            "",
            "<main>\0</main>",
            "<main>",
            "</main>",
            "<main onclick=\"bad()\">x</main>",
            "<main :hydrate=\"policy\">x</main>",
            "<main :=\"value\">x</main>",
            "<main :title=\"bad syntax\">x</main>",
            "<script>bad()</script>",
            "<web-widget hydrate=\"load\"></web-widget>",
            "<unknown>x</unknown>",
            "<logic>x</logic>",
            "<loop>x</loop>",
            "<for>x</for>",
            "<loop for=\"item of values\">x</loop>",
            "<loop :for=\"1item of values\">x</loop>",
            "<loop :for=\"item values\">x</loop>",
            "<slot name=\"named\"></slot>",
            "<p>{missing</p>",
            "<p>{!raw}</p>",
        ] {
            assert!(compile_without_components(source).is_err(), "{source}");
        }
    }

    #[test]
    fn every_control_form_comment_style_and_hydration_policy_parses() {
        let components = BTreeSet::from([String::from("demo-card")]);
        let source = r#"
<style>:root { color: red; }</style>
<!-- retained source comment -->
<p>\{literal\}</p>
<logic :if="false">a</logic>
<logic :else-if="true">b</logic>
<logic else>c</logic>
<if :when="false">d</if>
<else>e</else>
<loop :for="item of items">{item}</loop>
<for :each="item in items">{item}</for>
<demo-card hydrate="idle"></demo-card>
<demo-card hydrate="visible"></demo-card>
<demo-card hydrate="interaction"></demo-card>
<demo-card hydrate="never"></demo-card>
"#;
        let program = compile(source, &components).unwrap_or_else(|_| unreachable!());
        assert!(!program.nodes.is_empty());
        for policy in ["", "later", "LOAD"] {
            let source = format!("<demo-card hydrate=\"{policy}\"></demo-card>");
            assert!(compile(&source, &components).is_err());
        }
    }

    #[test]
    fn event_bindings_parse_into_markers_and_structured_payloads() {
        assert_eq!(
            binding_parts("click", "increment()"),
            (
                String::from("data-tac-on-click"),
                String::from("increment"),
                vec![]
            )
        );
        assert_eq!(
            binding_parts("click", "addBy(5)"),
            (
                String::from("data-tac-on-click"),
                String::from("addBy"),
                vec![EventArgument::Literal(String::from("5"))]
            )
        );
        assert_eq!(binding_parts("input", "rename").2, vec![]);
        assert_eq!(
            binding_parts("update:selected", "choose()").0,
            String::from("data-tac-on-update__selected")
        );
        assert_eq!(
            binding_parts("click", "setMode('dark', true, 2, null)").2,
            vec![
                EventArgument::Literal(String::from("\"dark\"")),
                EventArgument::Literal(String::from("true")),
                EventArgument::Literal(String::from("2")),
                EventArgument::Literal(String::from("null")),
            ]
        );
    }

    #[test]
    fn event_arguments_read_the_dispatched_event_and_the_render_scope() {
        assert_eq!(
            input_arguments("go($event)"),
            vec![EventArgument::EventPath(String::new())]
        );
        assert_eq!(
            input_arguments("setName($event.target.value)"),
            vec![EventArgument::EventPath(String::from("target.value"))]
        );
        assert_eq!(
            input_arguments("field('email', $event.target.value)"),
            vec![
                EventArgument::Literal(String::from("\"email\"")),
                EventArgument::EventPath(String::from("target.value")),
            ]
        );
        assert!(matches!(
            input_arguments("select(tab.id)").as_slice(),
            [EventArgument::Scope(_)]
        ));
        assert!(bind("input", "go($event.a.b.c.d.e)").is_err());
        assert!(bind("input", "go($event.)").is_err());
    }

    #[test]
    fn event_assignments_name_their_target_and_operator() {
        let (_, assignment) = bind("input", "query = $event.target.value").expect("assignment");
        let target = assignment.assign.expect("assignment");
        assert_eq!(target.target, "query");
        assert_eq!(target.operator, "=");
        assert_eq!(
            assignment.arguments,
            vec![EventArgument::EventPath(String::from("target.value"))]
        );

        let (_, compound) = bind("click", "$clicks += 1").expect("compound assignment");
        let target = compound.assign.expect("assignment");
        assert_eq!(target.target, "$clicks");
        assert_eq!(target.operator, "+=");

        let (_, comparison) = bind("click", "go(a == b)").expect("comparison");
        assert!(comparison.assign.is_none());
    }

    #[test]
    fn event_bindings_never_admit_executable_code() {
        for value in [
            "fetch('/x').then(r=>r.json())",
            "increment(); steal()",
            "a.b.c()",
            "a().b()",
            "() => 1",
            "increment(",
            "new Function('x')()",
            "",
        ] {
            assert!(bind("click", value).is_err(), "{value} was accepted");
        }
        assert!(bind("", "increment()").is_err());
    }

    #[test]
    fn event_handlers_may_name_one_service_member() {
        let (_, member) = bind("click", "basket.add()").expect("service member handler");
        assert_eq!(member.handler, "basket.add");
        let (_, assigned) =
            bind("click", "basket.query = $event.target.value").expect("service assignment");
        assert_eq!(assigned.assign.expect("assignment").target, "basket.query");
        let refused = bind("click", "a.b.c()").expect_err("two dots");
        assert!(
            refused.contains("Event handler must name one handler, as save() or basket.add()."),
            "{refused}"
        );
    }

    #[test]
    fn unsafe_handler_arguments_never_read_the_event() {
        for value in [
            "increment(unquoted)",
            "increment('a' + 'b')",
            "alert(document.cookie)",
        ] {
            let (_, binding) = bind("click", value).expect(value);
            let reads_event = binding
                .arguments
                .iter()
                .any(|argument| matches!(argument, EventArgument::EventPath(_)));
            assert!(!reads_event, "{value}");
        }
    }

    #[test]
    fn braces_in_a_plain_attribute_are_refused_with_a_span_on_the_attribute() {
        let components = BTreeSet::from([String::from("demo-card")]);
        for (source, attribute, help) in [
            (
                r#"<p title="{name}">x</p>"#,
                r#"title="{name}""#,
                r#"Write :title="name" to set it from a value."#,
            ),
            (
                r#"<p class="card {kind}">x</p>"#,
                r#"class="card {kind}""#,
                r#"Write :class="'card ' + kind" to join the text and the values."#,
            ),
            (
                r#"<div hidden="{drawer.open ? '' : 'hidden'}"></div>"#,
                r#"hidden="{drawer.open ? '' : 'hidden'}""#,
                r#"Write :hidden="condition" with a condition that is true or false. When the condition is false, Tachyon leaves the attribute out."#,
            ),
            (
                r#"<demo-card label="{title}"></demo-card>"#,
                r#"label="{title}""#,
                r#"Write :label="title" to set it from a value."#,
            ),
        ] {
            let error = compile(source, &components).expect_err(source);
            let found = error
                .diagnostics()
                .iter()
                .find(|value| value.code.number() == 1313)
                .expect(source);
            let span = found.spans.first().expect(source);
            let start = usize::try_from(span.start).expect(source);
            let end = usize::try_from(span.end).expect(source);
            assert_eq!(&source[start..end], attribute, "{source}");
            assert!(
                found.message.contains("braces work only in text"),
                "{source}"
            );
            assert_eq!(found.help.as_deref(), Some(help), "{source}");
        }
    }

    #[test]
    fn a_view_with_document_tags_is_refused_with_a_span_on_each_tag() {
        for (source, tags) in [
            (
                "<!doctype html><html><body><p>Hi</p></body></html>",
                vec!["<!doctype html>", "<html>", "<body>"],
            ),
            (
                "<!-- note --><!DOCTYPE html>\n<p>Hi</p>",
                vec!["<!DOCTYPE html>"],
            ),
            (
                "<html lang=\"en\"><head><title>Hi</title></head><main>Hi</main></html>",
                vec!["<html lang=\"en\">", "<head>"],
            ),
            ("<main><body>Hi</body></main>", vec!["<body>"]),
        ] {
            let error = compile_without_components(source).expect_err(source);
            let found: Vec<&str> = error
                .diagnostics()
                .iter()
                .filter(|value| value.code.number() == 1307)
                .map(|value| {
                    let span = value.spans.first().expect(source);
                    let start = usize::try_from(span.start).expect(source);
                    let end = usize::try_from(span.end).expect(source);
                    assert!(value.message.contains("Tachyon writes the document"));
                    assert!(
                        value
                            .help
                            .as_deref()
                            .is_some_and(|help| help.contains("client/manifest.json")),
                        "{source}"
                    );
                    &source[start..end]
                })
                .collect();
            assert_eq!(found, tags, "{source}");
        }
    }

    #[test]
    fn document_tags_shown_as_text_are_not_refused() {
        for source in [
            "<pre><code>&lt;!doctype html&gt;&lt;html&gt;&lt;head&gt;&lt;body&gt;</code></pre>",
            "<template><p>&lt;body&gt; is written by Tachyon.</p></template>",
            "<p>Tachyon writes &lt;html&gt; and &lt;head&gt;.</p>",
            "<table><thead><tr><th>A</th></tr></thead><tbody><tr><td>1</td></tr></tbody></table>",
            "<header>Top</header><!-- <html><body> -->",
        ] {
            assert!(compile_without_components(source).is_ok(), "{source}");
        }
    }

    #[test]
    fn literal_braces_handlers_and_style_text_are_not_refused() {
        for source in [
            r#"<p :title="'{name}'">x</p>"#,
            r#"<input pattern="[0-9]{3}" />"#,
            r#"<div data-config='{"theme": "dark"}'></div>"#,
            r#"<p title="\{name}">x</p>"#,
            r#"<button on:click="shell.refresh()">Go</button>"#,
            r#"<pre><code>&lt;p title="\{name}"&gt;</code></pre>"#,
            r#"<style>p[title="{name}"] { color: red; }</style>"#,
        ] {
            assert!(compile_without_components(source).is_ok(), "{source}");
        }
    }
}
