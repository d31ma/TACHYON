use super::piece::Piece;

pub(super) struct SchemaConstructor;

impl SchemaConstructor {
    const TYPE_MODIFIERS: [&'static str; 9] = [
        "public",
        "internal",
        "sealed",
        "partial",
        "readonly",
        "abstract",
        "private",
        "protected",
        "file",
    ];
    pub(super) fn first_refused(code: Piece<'_>, source: &str) -> Option<usize> {
        let without_constructor = Self::blanked(code, source);
        super::csharp::first_refused(Piece::new(code.at, &without_constructor))
    }

    fn blanked(code: Piece<'_>, source: &str) -> String {
        let mut bytes = code.text.as_bytes().to_vec();
        code.items()
            .into_iter()
            .for_each(|item| Self::blank_in_type(item, &mut bytes, code.at, source));
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn blank_in_type(item: Piece<'_>, bytes: &mut [u8], offset: usize, source: &str) {
        let declaration = item.after_annotations().after_words(&Self::TYPE_MODIFIERS);
        declaration
            .after_word("class")
            .and_then(Self::members)
            .into_iter()
            .flat_map(|(name, members)| members.into_iter().map(move |member| (name, member)))
            .filter(|(name, member)| Self::refuses_construction(*member, name, source))
            .for_each(|(_, member)| {
                let start = member.at - offset;
                crate::model_layout::JavaModelSchema::blank_bytes(
                    &mut bytes[start..start + member.text.len()],
                );
            });
    }

    fn members(class: Piece<'_>) -> Option<(&str, Vec<Piece<'_>>)> {
        Some((class.first_word(), class.body()?.inner.items()))
    }

    fn refuses_construction(member: Piece<'_>, name: &str, source: &str) -> bool {
        Self::constructor(member, name).is_some_and(|body| {
            ![
                body.head.text.trim() == "()",
                body.has_plain_end(),
                Self::throws_only(body.inner, source),
            ]
            .contains(&false)
        })
    }

    fn constructor<'a>(member: Piece<'a>, name: &str) -> Option<super::piece::Body<'a>> {
        member.after_word("private")?.after_word(name)?.body()
    }

    fn throws_only(inner: Piece<'_>, source: &str) -> bool {
        Self::thrown_argument(inner).is_some_and(|(name, argument, close)| {
            ![
                ["System.Exception", "Exception"].contains(&name),
                argument.slice(1, close).text.trim().is_empty(),
                Self::literal_argument(argument, close, source),
                argument.from(close + 1).trim().text == ";",
            ]
            .contains(&false)
        })
    }

    fn literal_argument(argument: Piece<'_>, close: usize, source: &str) -> bool {
        let literal = source[argument.at + 1..argument.at + close].trim();
        ![literal.starts_with('"'), literal.ends_with('"')].contains(&false)
    }

    fn thrown_argument(inner: Piece<'_>) -> Option<(&str, Piece<'_>, usize)> {
        let expression = inner.trim().after_word("throw")?.after_word("new")?;
        let name = expression.path();
        let argument = expression.from(name.len()).trim();
        let close = argument.closing(0).filter(|_| argument.starts_with("("))?;
        Some((name, argument, close))
    }
}
