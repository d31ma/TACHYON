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

    pub(super) fn after_operator(self, operator: &str) -> Self {
        Self::operand(self, operator)
    }

    pub(super) fn assigned(self, value: Self, compound: bool) -> Self {
        Self::assignment(self, value, compound)
    }

    pub(super) fn stored_at_member(self) -> Self {
        Self::member_value(self)
    }

    fn operand(kind: Self, operator: &str) -> Self {
        match (kind, operator) {
            (Self::Data, "|" | "&" | "?" | ":") => Self::Unknown,
            _ => kind,
        }
    }

    fn assignment(kind: Self, value: Self, compound: bool) -> Self {
        match (value, compound) {
            (Self::Data, true) => kind,
            _ => value,
        }
    }

    fn member_value(kind: Self) -> Self {
        match kind {
            Self::Data => Self::Unknown,
            _ => kind,
        }
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
            | (
                Self::Unknown | Self::Node | Self::CreatedNode,
                "insertAdjacentHTML" | "setHTMLUnsafe" | "setHTML",
            ) => Self::MarkupCall,
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
            )
            | (Self::MarkupCall | Self::StyleCall | Self::NodeInsertion, "call" | "apply") => {
                receiver
            }
            (Self::NodeFactory, "call" | "apply") => Self::CreatedNode,
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
