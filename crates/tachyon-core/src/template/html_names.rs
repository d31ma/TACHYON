#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HtmlNames;

impl HtmlNames {
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
        "code",
        "col",
        "colgroup",
        "data",
        "datalist",
        "dd",
        "del",
        "details",
        "dfn",
        "dialog",
        "div",
        "dl",
        "dt",
        "em",
        "embed",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
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
        "main",
        "map",
        "mark",
        "menu",
        "meta",
        "meter",
        "nav",
        "noscript",
        "object",
        "ol",
        "optgroup",
        "option",
        "output",
        "p",
        "picture",
        "pre",
        "progress",
        "q",
        "rp",
        "rt",
        "ruby",
        "s",
        "samp",
        "search",
        "section",
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
        "u",
        "ul",
        "var",
        "video",
        "wbr",
    ];
    const SVG_TAGS: &[&str] = &[
        "a",
        "animate",
        "animateMotion",
        "animateTransform",
        "circle",
        "clipPath",
        "defs",
        "desc",
        "discard",
        "ellipse",
        "feBlend",
        "feColorMatrix",
        "feComponentTransfer",
        "feComposite",
        "feConvolveMatrix",
        "feDiffuseLighting",
        "feDisplacementMap",
        "feDistantLight",
        "feDropShadow",
        "feFlood",
        "feFuncA",
        "feFuncB",
        "feFuncG",
        "feFuncR",
        "feGaussianBlur",
        "feImage",
        "feMerge",
        "feMergeNode",
        "feMorphology",
        "feOffset",
        "fePointLight",
        "feSpecularLighting",
        "feSpotLight",
        "feTile",
        "feTurbulence",
        "filter",
        "foreignObject",
        "g",
        "image",
        "line",
        "linearGradient",
        "marker",
        "mask",
        "metadata",
        "mpath",
        "path",
        "pattern",
        "polygon",
        "polyline",
        "radialGradient",
        "rect",
        "set",
        "stop",
        "style",
        "svg",
        "symbol",
        "text",
        "textPath",
        "title",
        "tspan",
        "use",
        "view",
    ];
    const MATHML_TAGS: &[&str] = &[
        "annotation",
        "annotation-xml",
        "maction",
        "math",
        "merror",
        "mfrac",
        "mi",
        "mmultiscripts",
        "mn",
        "mo",
        "mover",
        "mpadded",
        "mphantom",
        "mprescripts",
        "mroot",
        "mrow",
        "ms",
        "mspace",
        "msqrt",
        "mstyle",
        "msub",
        "msubsup",
        "msup",
        "mtable",
        "mtd",
        "mtext",
        "mtr",
        "munder",
        "munderover",
        "none",
        "semantics",
    ];

    pub(crate) fn is_standard_tag(name: &str) -> bool {
        Self::standard_tags().any(|tag| tag.eq_ignore_ascii_case(name))
    }

    pub(crate) fn is_event_handler_attribute(name: &str) -> bool {
        Self::EVENT_HANDLER_ATTRIBUTES.contains(&name)
    }

    fn standard_tags() -> impl Iterator<Item = &'static &'static str> {
        Self::HTML_TAGS
            .iter()
            .chain(Self::SVG_TAGS)
            .chain(Self::MATHML_TAGS)
    }

    pub(crate) fn is_custom_element(name: &str) -> bool {
        if !name.contains('-') || name.starts_with('-') || name.ends_with('-') {
            return false;
        }
        name.bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::HtmlNames;
    use std::collections::BTreeSet;

    const RUNTIME: &str = include_str!("tac-client.js");
    const RUNTIME_MAP_START: &str = "static #svgTags = new Map([";
    const RUNTIME_MAP_END: &str = "].map(";
    const FOREIGN_OBJECT_CONSTANT: &str = "Elements.#foreignObjectTag";

    fn runtime_svg_case_map() -> BTreeSet<String> {
        let start = RUNTIME.find(RUNTIME_MAP_START).expect("svg tag map") + RUNTIME_MAP_START.len();
        let length = RUNTIME[start..]
            .find(RUNTIME_MAP_END)
            .expect("svg tag map end");
        let block =
            RUNTIME[start..start + length].replace(FOREIGN_OBJECT_CONSTANT, "'foreignObject'");
        block
            .split('\'')
            .skip(1)
            .step_by(2)
            .map(String::from)
            .collect()
    }

    fn mixed_case_svg_tags() -> BTreeSet<String> {
        HtmlNames::SVG_TAGS
            .iter()
            .filter(|tag| tag.bytes().any(|byte| byte.is_ascii_uppercase()))
            .map(|tag| String::from(*tag))
            .collect()
    }

    #[test]
    fn standard_tags_event_attributes_and_custom_elements_are_recognized() {
        assert!(HtmlNames::is_standard_tag("main"));
        assert!(HtmlNames::is_standard_tag("path"));
        assert!(!HtmlNames::is_standard_tag("hero"));
        assert!(HtmlNames::is_event_handler_attribute("onclick"));
        assert!(!HtmlNames::is_event_handler_attribute("onboarding"));
        assert!(HtmlNames::is_custom_element("user-card"));
        for name in ["hero", "-bad", "bad-", "User-card", "user_card"] {
            assert!(!HtmlNames::is_custom_element(name), "{name}");
        }
    }

    #[test]
    fn every_svg_2_and_mathml_core_element_is_a_standard_tag() {
        for name in [
            "stop",
            "lineargradient",
            "radialgradient",
            "pattern",
            "mask",
            "marker",
            "filter",
            "fegaussianblur",
            "fedropshadow",
            "tspan",
            "textpath",
            "image",
            "animate",
            "mfrac",
            "msqrt",
            "annotation-xml",
        ] {
            assert!(HtmlNames::is_standard_tag(name), "{name}");
        }
    }

    #[test]
    fn tac_control_tags_stay_out_of_the_standard_tags() {
        for name in ["switch", "script", "logic", "loop", "case"] {
            assert!(!HtmlNames::is_standard_tag(name), "{name}");
        }
    }

    #[test]
    fn runtime_svg_case_map_matches_the_mixed_case_svg_tags() {
        assert_eq!(runtime_svg_case_map(), mixed_case_svg_tags());
    }

    #[test]
    fn each_tag_list_names_each_tag_once() {
        for list in [
            HtmlNames::HTML_TAGS,
            HtmlNames::SVG_TAGS,
            HtmlNames::MATHML_TAGS,
        ] {
            let unique = list.iter().collect::<BTreeSet<_>>();
            assert_eq!(unique.len(), list.len());
        }
    }
}
