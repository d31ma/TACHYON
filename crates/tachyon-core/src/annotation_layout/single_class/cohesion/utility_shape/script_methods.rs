use super::Words;

pub(super) struct ScriptMethods;

impl ScriptMethods {
    const MODIFIERS: [&'static str; 12] = [
        "static",
        "private",
        "public",
        "protected",
        "async",
        "get",
        "set",
        "#",
        "*",
        "override",
        "abstract",
        "declare",
    ];

    pub(super) fn start(words: &Words<'_>, head: usize) -> usize {
        (0..head)
            .rev()
            .take_while(|at| Self::prefix(words, *at, head))
            .last()
            .unwrap_or(head)
    }

    fn prefix(words: &Words<'_>, at: usize, head: usize) -> bool {
        [
            words.depth(at) == words.depth(head),
            Self::MODIFIERS.contains(&words.text(at)),
        ]
        .into_iter()
        .all(|holds| holds)
    }
}
