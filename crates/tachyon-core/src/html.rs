use crate::Failure;
use crate::failure::{diagnostic, source_span};
use html5gum::{DefaultEmitter, StartTag, Token, Tokenizer};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HtmlDocument {
    content: String,
}

impl HtmlDocument {
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HtmlFrontend;

impl HtmlFrontend {
    const MAX_HTML_BYTES: u64 = 1_048_576;
    const CONTROL_TAGS: &[&str] = &["else", "for", "if", "loop"];
    const EVENT_HANDLER_ATTRIBUTES: &[&str] = &[
        "onabort",
        "onbeforeinput",
        "onblur",
        "oncancel",
        "oncanplay",
        "onchange",
        "onclick",
        "onclose",
        "oncontextmenu",
        "oncopy",
        "oncut",
        "ondblclick",
        "ondrag",
        "ondragend",
        "ondragenter",
        "ondragleave",
        "ondragover",
        "ondragstart",
        "ondrop",
        "onerror",
        "onfocus",
        "oninput",
        "oninvalid",
        "onkeydown",
        "onkeypress",
        "onkeyup",
        "onload",
        "onmousedown",
        "onmouseenter",
        "onmouseleave",
        "onmousemove",
        "onmouseout",
        "onmouseover",
        "onmouseup",
        "onpaste",
        "onpointercancel",
        "onpointerdown",
        "onpointerenter",
        "onpointerleave",
        "onpointermove",
        "onpointerout",
        "onpointerover",
        "onpointerup",
        "onreset",
        "onscroll",
        "onselect",
        "onsubmit",
        "ontoggle",
        "ontouchend",
        "ontouchmove",
        "ontouchstart",
        "onwheel",
    ];
    const HTML_TAGS: &[&str] = &[
        "a",
        "abbr",
        "address",
        "area",
        "article",
        "aside",
        "audio",
        "b",
        "base",
        "bdi",
        "bdo",
        "blockquote",
        "body",
        "br",
        "button",
        "canvas",
        "caption",
        "cite",
        "circle",
        "code",
        "col",
        "colgroup",
        "data",
        "datalist",
        "dd",
        "del",
        "details",
        "defs",
        "dfn",
        "dialog",
        "div",
        "dl",
        "dt",
        "em",
        "embed",
        "ellipse",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "foreignobject",
        "g",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "head",
        "header",
        "hgroup",
        "hr",
        "html",
        "i",
        "iframe",
        "img",
        "input",
        "ins",
        "kbd",
        "label",
        "legend",
        "li",
        "link",
        "line",
        "lineargradient",
        "main",
        "map",
        "mark",
        "math",
        "menu",
        "meta",
        "meter",
        "mi",
        "mn",
        "mo",
        "ms",
        "mtext",
        "nav",
        "noscript",
        "object",
        "ol",
        "optgroup",
        "option",
        "output",
        "p",
        "path",
        "picture",
        "polygon",
        "polyline",
        "pre",
        "progress",
        "q",
        "rp",
        "rt",
        "ruby",
        "rect",
        "s",
        "samp",
        "search",
        "section",
        "semantics",
        "select",
        "slot",
        "small",
        "source",
        "span",
        "strong",
        "style",
        "sub",
        "summary",
        "sup",
        "svg",
        "symbol",
        "table",
        "tbody",
        "td",
        "template",
        "textarea",
        "tfoot",
        "th",
        "thead",
        "time",
        "title",
        "tr",
        "track",
        "text",
        "u",
        "ul",
        "use",
        "var",
        "video",
        "wbr",
    ];

    pub fn parse_file(path: &Path, source_path: &str) -> Result<HtmlDocument, Failure> {
        let bytes = read_bounded(path, source_path)?;
        let source = String::from_utf8(bytes)
            .map_err(|error| invalid_utf8_failure(source_path, error.utf8_error().valid_up_to()))?;
        Self::parse(&source, source_path)
    }

    pub fn parse(source: &str, source_path: &str) -> Result<HtmlDocument, Failure> {
        if source.trim().is_empty() {
            return Err(Failure::one(diagnostic(
                1107,
                format!("HTML source '{source_path}' is empty."),
                Some(String::from("Add at least one semantic HTML element.")),
                source_span(source_path, 0, source.len()),
            )));
        }
        if let Some(index) = source.find('\0') {
            return Err(Failure::one(diagnostic(
                1102,
                format!("HTML source '{source_path}' contains a NUL byte."),
                Some(String::from(
                    "Remove the NUL byte and save the source as UTF-8.",
                )),
                source_span(source_path, index, index + 1),
            )));
        }

        let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
        let analysis = analyze(&normalized, source_path)?;
        let content = if analysis.is_document {
            complete_document(&normalized, analysis.has_doctype)
        } else {
            wrap_fragment(&normalized)
        };
        Ok(HtmlDocument { content })
    }
}

fn invalid_utf8_failure(source_path: &str, invalid: usize) -> Failure {
    Failure::one(diagnostic(
        1102,
        format!("HTML source '{source_path}' is not valid UTF-8."),
        Some(String::from("Save Tachyon HTML sources as UTF-8.")),
        source_span(source_path, invalid, invalid.saturating_add(1)),
    ))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct HtmlAnalysis {
    has_doctype: bool,
    is_document: bool,
}

fn read_bounded(path: &Path, source_path: &str) -> Result<Vec<u8>, Failure> {
    let limit = HtmlFrontend::MAX_HTML_BYTES;
    let file = File::open(path).map_err(|error| html_read_error(source_path, &error))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| html_read_error(source_path, &error))?;
    if bytes.len() as u64 > limit {
        return Err(Failure::one(diagnostic(
            1101,
            format!("HTML source '{source_path}' exceeds the 1 MiB Phase 1 limit."),
            Some(String::from("Reduce the view below 1 MiB.")),
            source_span(source_path, 0, bytes.len()),
        )));
    }
    Ok(bytes)
}

fn html_read_error(source_path: &str, error: &std::io::Error) -> Failure {
    Failure::one(diagnostic(
        1101,
        format!("Cannot read HTML source '{source_path}': {error}"),
        None,
        source_span(source_path, 0, 0),
    ))
}

fn analyze(source: &str, source_path: &str) -> Result<HtmlAnalysis, Failure> {
    let emitter = DefaultEmitter::<usize>::new_with_span();
    let tokenizer = Tokenizer::new_with_emitter(source, emitter);
    let mut analysis = HtmlAnalysis::default();
    for result in tokenizer {
        let token = match result {
            Ok(token) => token,
            Err(infallible) => match infallible {},
        };
        match token {
            Token::Doctype(_) => {
                analysis.has_doctype = true;
                analysis.is_document = true;
            }
            Token::StartTag(tag) => {
                if check_start_tag(&tag, source_path)? {
                    analysis.is_document = true;
                }
            }
            Token::Error(error) => {
                return Err(tokenizer_failure(
                    source_path,
                    error.span.start,
                    error.span.end,
                ));
            }
            Token::EndTag(_) | Token::String(_) | Token::Comment(_) => {}
        }
    }
    Ok(analysis)
}

fn check_start_tag(tag: &StartTag<usize>, source_path: &str) -> Result<bool, Failure> {
    let name = String::from_utf8_lossy(&tag.name).into_owned();
    let is_html_root = name == "html";
    validate_tag(&name, tag.span.start, tag.span.end, source_path)?;
    for attribute in tag.attributes.keys() {
        let attribute = String::from_utf8_lossy(attribute);
        if is_event_handler_attribute(&attribute) {
            return Err(deferred_dynamic_html(
                source_path,
                tag.span.start,
                tag.span.end,
            ));
        }
    }
    Ok(is_html_root)
}

fn tokenizer_failure(source_path: &str, start: usize, end: usize) -> Failure {
    Failure::one(diagnostic(
        1104,
        format!("HTML source '{source_path}' contains a tokenizer error."),
        Some(String::from(
            "Correct the malformed token before building the page.",
        )),
        source_span(source_path, start, end),
    ))
}

fn validate_tag(name: &str, start: usize, end: usize, source_path: &str) -> Result<(), Failure> {
    if HtmlFrontend::CONTROL_TAGS.contains(&name) {
        return Err(Failure::one(diagnostic(
            1103,
            format!("Control tag '<{name}>' is not available until Phase 3."),
            Some(String::from(
                "Use static HTML in the Phase 1 vertical slice.",
            )),
            source_span(source_path, start, end),
        )));
    }
    if name == "script" {
        return Err(deferred_dynamic_html(source_path, start, end));
    }
    if !is_standard_html_tag(name) && !valid_custom_element(name) {
        return Err(Failure::one(diagnostic(
            1106,
            format!("Tac component '<{name}>' is not available until Phase 3."),
            Some(String::from(
                "Use a standard HTML tag or a hyphenated web-component name.",
            )),
            source_span(source_path, start, end),
        )));
    }
    Ok(())
}

fn deferred_dynamic_html(source_path: &str, start: usize, end: usize) -> Failure {
    Failure::one(diagnostic(
        1105,
        "Inline executable HTML is not supported in Phase 1.",
        Some(String::from(
            "Use static semantic HTML; controller execution arrives in Phase 3.",
        )),
        source_span(source_path, start, end),
    ))
}

pub(crate) fn is_standard_html_tag(name: &str) -> bool {
    HtmlFrontend::HTML_TAGS.contains(&name)
}

pub(crate) fn is_event_handler_attribute(name: &str) -> bool {
    HtmlFrontend::EVENT_HANDLER_ATTRIBUTES.contains(&name)
}

pub(crate) fn valid_custom_element(name: &str) -> bool {
    if !name.contains('-') || name.starts_with('-') || name.ends_with('-') {
        return false;
    }
    name.bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn complete_document(source: &str, has_doctype: bool) -> String {
    let source = source.trim();
    if has_doctype {
        format!("{source}\n")
    } else {
        format!("<!doctype html>\n{source}\n")
    }
}

fn wrap_fragment(source: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>Tachyon</title>\n</head>\n<body>\n{}\n</body>\n</html>\n",
        source.trim()
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{HtmlDocument, HtmlFrontend};
    use crate::Failure;
    use std::fs;
    use std::path::Path;

    fn parse(source: &str) -> Result<HtmlDocument, Failure> {
        HtmlFrontend::parse(source, "client/pages/tac.html")
    }

    fn file_error(path: &Path, reason: &str) -> String {
        HtmlFrontend::parse_file(path, "client/pages/tac.html")
            .expect_err(reason)
            .to_string()
    }

    #[test]
    fn fragments_are_wrapped_and_line_endings_are_stable() {
        let document = parse("<main>\r\nHello</main>\r\n").expect("valid HTML");
        assert!(document.content().starts_with("<!doctype html>"));
        assert!(document.content().contains("<main>\nHello</main>"));
        assert!(!document.content().contains('\r'));
    }

    #[test]
    fn complete_documents_are_preserved_with_a_doctype() {
        let document = parse("<html><head><title>X</title></head><body>X</body></html>")
            .expect("valid document");
        assert!(document.content().starts_with("<!doctype html>\n<html>"));
    }

    #[test]
    fn dynamic_and_unknown_tags_are_rejected() {
        for (source, code) in [
            ("<if>Later</if>", "TY1103"),
            ("<script>bad()</script>", "TY1105"),
            ("<hero>Later</hero>", "TY1106"),
        ] {
            let error = parse(source).expect_err("deferred syntax");
            assert!(error.to_string().contains(code));
        }
        assert!(parse("<user-card></user-card>").is_ok());
        assert!(
            parse("<svg><path d=\"M0 0\"></path></svg><div onboarding=\"done\"></div>").is_ok()
        );
    }

    #[test]
    fn empty_nul_event_and_tokenizer_errors_are_rejected() {
        for (source, code) in [
            (" \n\t", "TY1107"),
            ("<main>\0</main>", "TY1102"),
            ("<button onclick=\"go()\">Go</button>", "TY1105"),
            ("<div a=\"first\" a=\"second\">Duplicate</div>", "TY1104"),
            ("<-bad></-bad>", "TY1104"),
            ("<bad-></bad->", "TY1106"),
        ] {
            let error = parse(source).expect_err("invalid HTML");
            assert!(error.to_string().contains(code), "{source}: {error}");
        }
    }

    #[test]
    fn file_reads_are_bounded_and_utf8_only() {
        let root = tempfile::tempdir().expect("workspace");
        let missing = root.path().join("missing.html");
        assert!(file_error(&missing, "missing").contains("TY1101"));

        let invalid = root.path().join("invalid.html");
        fs::write(&invalid, [0xff, 0xfe]).expect("invalid UTF-8 fixture");
        assert!(file_error(&invalid, "UTF-8").contains("TY1102"));

        let large = root.path().join("large.html");
        fs::write(&large, vec![b'x'; 1_048_577]).expect("large fixture");
        assert!(file_error(&large, "size").contains("TY1101"));
    }
}
