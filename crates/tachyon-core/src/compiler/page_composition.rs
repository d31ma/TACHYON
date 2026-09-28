use crate::failure::{diagnostic, source_span};
use crate::template::{TemplateNode, TemplateNodeKind, TemplateProgram, TextPart};
use tachyon_diagnostics::Diagnostic;

pub(super) struct PageComposition;

impl PageComposition {
    pub(super) fn diagnostics(
        program: &TemplateProgram,
        inline_state: &str,
        source_path: &str,
    ) -> Vec<Diagnostic> {
        let mut found: Vec<Diagnostic> = Self::state_script_failure(inline_state, source_path)
            .into_iter()
            .collect();
        Self::collect_page_markup(&program.nodes, source_path, &mut found);
        found
    }

    pub(super) fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
        diagnostics.sort_by(|left, right| {
            let left_span = left.spans.first();
            let right_span = right.spans.first();
            left_span
                .map(|span| (&span.file, span.start, span.end))
                .cmp(&right_span.map(|span| (&span.file, span.start, span.end)))
                .then_with(|| left.code.cmp(&right.code))
        });
    }

    fn state_script_failure(inline_state: &str, source_path: &str) -> Option<Diagnostic> {
        let has_state = !inline_state.trim().is_empty();
        has_state.then(|| Self::failure(source_path, 0, 0, "a page state script"))
    }

    fn collect_page_markup(nodes: &[TemplateNode], source_path: &str, found: &mut Vec<Diagnostic>) {
        for node in nodes {
            Self::check_node(node, source_path, found);
        }
    }

    fn check_node(node: &TemplateNode, source_path: &str, found: &mut Vec<Diagnostic>) {
        match &node.kind {
            TemplateNodeKind::Component { children, .. } => {
                Self::collect_page_markup(children, source_path, found);
            }
            _ => found.extend(Self::markup_failure(node, source_path)),
        }
    }

    fn markup_failure(node: &TemplateNode, source_path: &str) -> Option<Diagnostic> {
        let what = Self::markup_name(&node.kind)?;
        Some(Self::failure(
            source_path,
            node.range.start,
            node.range.end,
            what,
        ))
    }

    fn markup_name(kind: &TemplateNodeKind) -> Option<&'static str> {
        match kind {
            TemplateNodeKind::Component { .. } | TemplateNodeKind::Comment(_) => None,
            TemplateNodeKind::Text(parts) if Self::is_blank_text(parts) => None,
            TemplateNodeKind::Text(_) => Some("text or an expression"),
            TemplateNodeKind::Element { .. } => Some("a plain element"),
            TemplateNodeKind::Slot => Some("a slot"),
            _ => Some("control flow"),
        }
    }

    fn is_blank_text(parts: &[TextPart]) -> bool {
        parts.iter().all(Self::is_blank_part)
    }

    fn is_blank_part(part: &TextPart) -> bool {
        matches!(part, TextPart::Literal(text, _) if text.trim().is_empty())
    }

    fn failure(source_path: &str, start: usize, end: usize, what: &str) -> Diagnostic {
        diagnostic(
            1309,
            format!("Pages compose components: {what} cannot appear in a page."),
            Some(String::from(
                "A page's tac.html holds only component tags (nested as needed), whitespace and comments. Move markup, text and expressions into a component under client/components/ and place its tag here.",
            )),
            source_span(source_path, start, end),
        )
    }
}
