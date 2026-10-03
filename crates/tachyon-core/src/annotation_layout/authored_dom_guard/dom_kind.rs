#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum DomKind {
    #[default]
    Unknown,
    Data,
    Window,
    Document,
    Node,
    CreatedNode,
    Style,
    Sheet,
    MarkupCall,
    StyleCall,
    NodeFactory,
    NodeInsertion,
}

impl DomKind {
    pub(super) fn global(name: &str) -> Self {
        Self::named_global(name)
    }

    pub(super) fn member(self, name: &str) -> Self {
        Self::named_member(self, name)
    }

    pub(super) fn called(self, name: &str) -> Self {
        Self::named_call(self, name)
    }

    pub(super) fn invoked(self) -> Self {
        Self::invocation(self)
    }

    fn invocation(kind: Self) -> Self {
        match kind {
            Self::NodeFactory => Self::CreatedNode,
            _ => kind,
        }
    }

    fn named_global(name: &str) -> Self {
        match name {
            "document" => Self::Document,
            "window" | "self" | "globalThis" => Self::Window,
            "CSSStyleSheet" => Self::Sheet,
            _ => Self::Unknown,
        }
    }

    fn named_member(receiver: Self, name: &str) -> Self {
        match (receiver, name) {
            (Self::Document, "write" | "writeln")
            | (Self::Unknown | Self::Node | Self::CreatedNode, "insertAdjacentHTML") => {
                Self::MarkupCall
            }
            (Self::Document, "createElement" | "createElementNS" | "createDocumentFragment") => {
                Self::NodeFactory
            }
            (
                Self::Unknown | Self::Sheet,
                "insertRule" | "deleteRule" | "replaceSync" | "addRule" | "removeRule",
            )
            | (Self::Sheet, "replace")
            | (Self::Style, "setProperty" | "removeProperty") => Self::StyleCall,
            (
                Self::Unknown | Self::Node | Self::CreatedNode,
                "append"
                | "appendChild"
                | "prepend"
                | "before"
                | "after"
                | "replaceChildren"
                | "replaceChild"
                | "insertBefore"
                | "insertAdjacentElement",
            ) => Self::NodeInsertion,
            (Self::Window, "document") => Self::Document,
            (Self::Document, "body" | "head" | "documentElement") => Self::Node,
            (Self::Document, "styleSheets" | "adoptedStyleSheets")
            | (Self::Node | Self::CreatedNode, "sheet") => Self::Sheet,
            (Self::Node | Self::CreatedNode | Self::Unknown, "style") => Self::Style,
            (Self::Data, _) => Self::Data,
            _ => Self::Unknown,
        }
    }

    fn named_call(receiver: Self, name: &str) -> Self {
        match (receiver, name) {
            (
                Self::MarkupCall | Self::StyleCall | Self::NodeFactory | Self::NodeInsertion,
                "bind",
            ) => receiver,
            (Self::Document, "createElement" | "createElementNS" | "createDocumentFragment") => {
                Self::CreatedNode
            }
            (
                Self::Document | Self::Node | Self::CreatedNode,
                "querySelector"
                | "querySelectorAll"
                | "getElementById"
                | "getElementsByTagName"
                | "getElementsByClassName",
            ) => Self::Node,
            (Self::CreatedNode, "cloneNode") => Self::CreatedNode,
            _ => Self::Unknown,
        }
    }
}
