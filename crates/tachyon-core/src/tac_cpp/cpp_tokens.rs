use super::CppToken;

pub(crate) struct CppTokens {
    pub(crate) list: Vec<CppToken>,
    partners: Vec<Option<usize>>,
}

impl CppTokens {
    const OPENERS: [&'static str; 3] = ["(", "[", "{"];
    const CLOSERS: [&'static str; 3] = [")", "]", "}"];

    pub(crate) fn new(list: Vec<CppToken>) -> Self {
        let partners = Self::paired(&list);
        Self { list, partners }
    }

    pub(crate) fn len(&self) -> usize {
        self.list.len()
    }

    pub(crate) fn is(&self, index: usize, text: &str) -> bool {
        self.list.get(index).is_some_and(|token| token.is(text))
    }

    pub(crate) fn text(&self, index: usize) -> &str {
        self.list.get(index).map_or("", |token| token.text.as_str())
    }

    pub(crate) fn start(&self, index: usize) -> usize {
        self.list.get(index).map_or(usize::MAX, |token| token.start)
    }

    pub(crate) fn end(&self, index: usize) -> usize {
        self.list.get(index).map_or(usize::MAX, CppToken::end)
    }

    pub(crate) fn partner(&self, index: usize) -> Option<usize> {
        self.partners.get(index).copied().flatten()
    }

    pub(crate) fn skipped(&self, index: usize) -> usize {
        self.partner(index)
            .filter(|partner| *partner > index)
            .map_or(index, |partner| partner)
            + 1
    }

    pub(crate) fn is_word(&self, index: usize) -> bool {
        self.list.get(index).is_some_and(CppToken::is_word)
    }

    pub(crate) fn opens_attribute(&self, index: usize) -> bool {
        let second = self.list.get(index + 1);
        let touching = second.is_some_and(|next| self.list[index].touches(next));
        [self.is(index, "["), self.is(index + 1, "["), touching] == [true, true, true]
    }

    fn paired(list: &[CppToken]) -> Vec<Option<usize>> {
        let mut partners = vec![None; list.len()];
        let mut open = Vec::new();
        for (index, token) in list.iter().enumerate() {
            Self::pair(&mut partners, &mut open, index, token);
        }
        partners
    }

    fn pair(partners: &mut [Option<usize>], open: &mut Vec<usize>, index: usize, token: &CppToken) {
        let opens = Self::OPENERS.contains(&token.text.as_str());
        let closes = Self::CLOSERS.contains(&token.text.as_str());
        match (opens, closes, open.pop()) {
            (true, _, previous) => {
                open.extend(previous);
                open.push(index);
            }
            (_, true, Some(start)) => {
                partners[start] = Some(index);
                partners[index] = Some(start);
            }
            (_, _, previous) => open.extend(previous),
        }
    }
}
