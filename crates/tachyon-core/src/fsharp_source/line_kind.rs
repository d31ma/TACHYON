#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineKind {
    Blank,
    Open,
    Attribute,
    Type,
    Exception,
    Member,
    MemberValue,
    Constructor,
    Let,
    Do,
    Inherit,
    Interface,
    UnionCase,
    RecordStart,
    Directive,
    Other,
}

impl LineKind {
    const SYMBOLS: [(&'static str, Self); 5] = [
        ("[<assembly", Self::Directive),
        ("[<", Self::Attribute),
        ("|", Self::UnionCase),
        ("{", Self::RecordStart),
        ("#", Self::Directive),
    ];

    pub(crate) fn of(trimmed: &str) -> Self {
        let rest = trimmed.strip_prefix("static ").unwrap_or(trimmed);
        let first = rest.split([' ', '(']).next().unwrap_or("");
        let second = rest.split_whitespace().nth(1).unwrap_or("");
        Self::from_words(rest, first, second)
    }

    fn from_words(rest: &str, first: &str, second: &str) -> Self {
        match (first, second) {
            ("", _) => Self::Blank,
            ("open", _) => Self::Open,
            ("type" | "and", _) => Self::Type,
            ("exception", _) => Self::Exception,
            ("member", "val") => Self::MemberValue,
            ("member" | "override" | "default" | "abstract", _) => Self::Member,
            ("new", _) => Self::Constructor,
            ("let", _) => Self::Let,
            ("do", _) => Self::Do,
            ("inherit", _) => Self::Inherit,
            ("interface", _) => Self::Interface,
            ("namespace" | "module", _) => Self::Directive,
            _ => Self::symbol(rest),
        }
    }

    fn symbol(rest: &str) -> Self {
        Self::SYMBOLS
            .iter()
            .find(|(prefix, _)| rest.starts_with(prefix))
            .map_or(Self::Other, |(_, kind)| *kind)
    }
}

#[cfg(test)]
mod tests {
    use super::LineKind;

    #[test]
    fn lines_are_known_by_their_first_words() {
        assert_eq!(LineKind::of(""), LineKind::Blank);
        assert_eq!(LineKind::of("open Models"), LineKind::Open);
        assert_eq!(LineKind::of("[<Service>]"), LineKind::Attribute);
        assert_eq!(LineKind::of("[<assembly: X>]"), LineKind::Directive);
        assert_eq!(
            LineKind::of("member val Count = 0 with get, set"),
            LineKind::MemberValue
        );
        assert_eq!(LineKind::of("member this.Count = 0"), LineKind::Member);
        assert_eq!(LineKind::of("static member GET() = 1"), LineKind::Member);
        assert_eq!(LineKind::of("do"), LineKind::Do);
        assert_eq!(LineKind::of("done"), LineKind::Other);
        assert_eq!(LineKind::of("| Active"), LineKind::UnionCase);
        assert_eq!(LineKind::of("Price: float }"), LineKind::Other);
    }
}
