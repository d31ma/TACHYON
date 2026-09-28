use super::method::Method;
use super::words::Words;

pub(super) struct EndedMembers<'r, 'a> {
    words: &'r Words<'a>,
    methods: Vec<Method>,
    hidden: bool,
    private_names: Vec<String>,
}

impl<'r, 'a> EndedMembers<'r, 'a> {
    const SECTIONS: &'static [&'static str] = &["private", "protected", "public"];
    const OPENERS: &'static [&'static str] =
        &["def", "class", "module", "begin", "case", "do", "for"];
    const STATEMENT_OPENERS: &'static [&'static str] = &["if", "unless", "while", "until"];

    pub(super) fn read(words: &'r Words<'a>) -> Vec<Method> {
        let mut reader = Self {
            words,
            methods: Vec::new(),
            hidden: false,
            private_names: Vec::new(),
        };
        reader.read_all();
        reader.hide_listed_names();
        reader.methods
    }

    fn read_all(&mut self) {
        for at in 0..self.words.len() {
            self.note_section(at);
            self.note_private_list(at);
            self.note_method(at);
        }
    }

    fn note_section(&mut self, at: usize) {
        let text = self.words.text(at);
        let bare = self.words.line_start(at) & self.words.line_start(at + 1);
        let change = (Self::SECTIONS.contains(&text) & bare).then_some(text != "public");
        self.hidden = change.unwrap_or(self.hidden);
    }

    fn note_private_list(&mut self, at: usize) {
        let lists = (self.words.text(at) == "private") & (self.words.text(at + 1) == ":");
        if lists {
            self.collect_private_list(at);
        }
    }

    fn collect_private_list(&mut self, at: usize) {
        let end = self.words.line_end(at);
        let names: Vec<String> = (at + 1..end)
            .filter(|&index| self.words.text(index.wrapping_sub(1)) == ":")
            .map(|index| self.words.text(index).to_owned())
            .collect();
        self.private_names.extend(names);
    }

    fn note_method(&mut self, at: usize) {
        let is_method = (self.words.text(at) == "def") & !self.words.after_member_access(at);
        if is_method {
            self.add_method(at);
        }
    }

    fn add_method(&mut self, at: usize) {
        let on_self = (self.words.text(at + 1) == "self") & (self.words.text(at + 2) == ".");
        let name_at = at + 1 + 2 * usize::from(on_self);
        let name = self.words.text(name_at);
        let marked = self.words.text(at.wrapping_sub(1)) == "private";
        self.methods.push(Method {
            name: name.to_owned(),
            head: at,
            public: !(self.hidden | marked | name.starts_with('_')),
            body: name_at + 1..self.body_end(at, name_at),
            receiver: String::new(),
            nested: Vec::new(),
        });
    }

    fn body_end(&self, at: usize, name_at: usize) -> usize {
        let signature_end = self.words.after_group(name_at + 1);
        let endless = (self.words.text(signature_end) == "=") & self.words.spaced(signature_end);
        if endless {
            return self.words.line_end(name_at);
        }
        Self::block_end(self.words, at)
    }

    pub(super) fn block_end(words: &Words<'_>, at: usize) -> usize {
        let mut depth = 0_isize;
        (at..words.len())
            .find(|&index| {
                depth += Self::depth_change(words, index);
                depth == 0
            })
            .unwrap_or(words.len())
    }

    fn depth_change(words: &Words<'_>, at: usize) -> isize {
        let text = words.text(at);
        let plain = !words.after_member_access(at);
        let statement = words.line_start(at) | (words.text(at.wrapping_sub(1)) == "=");
        let opens =
            Self::OPENERS.contains(&text) | (Self::STATEMENT_OPENERS.contains(&text) & statement);
        let closes = text == "end";
        isize::from(opens & plain) - isize::from(closes & plain)
    }

    fn hide_listed_names(&mut self) {
        for method in &mut self.methods {
            method.public &= !self.private_names.contains(&method.name);
        }
    }
}
