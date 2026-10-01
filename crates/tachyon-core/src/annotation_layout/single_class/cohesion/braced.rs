use super::container::Container;
use super::head::Head;
use super::method::Method;
use super::spelling::Spelling;
use super::words::Words;
use std::ops::Range;

pub(super) struct BracedMembers<'r, 'a> {
    words: &'r Words<'a>,
    spelling: &'r Spelling,
    methods: Vec<Method>,
}

impl<'r, 'a> BracedMembers<'r, 'a> {
    const BODY_OPENERS: &'static [&'static str] = &["{", "=", ";"];
    const RUST_BODY_OPENERS: &'static [&'static str] = &["{", ";"];
    const CONTAINER_BLOCKERS: &'static [&'static str] =
        &["->", ">", ":", "(", ",", "&", "<", "=", "."];
    const NOT_BEFORE_NAME: &'static [&'static str] = &[
        ".", "new", "=", "@", "(", ",", "return", "await", "throw", "!", ":",
    ];
    const RESERVED: &'static [&'static str] = &[
        "if", "for", "foreach", "while", "switch", "catch", "return", "new", "super", "this",
        "function", "typeof", "await", "yield", "base", "else", "do", "try", "using", "lock",
    ];

    pub(super) fn read(words: &'r Words<'a>, spelling: &'r Spelling) -> Vec<Method> {
        let mut reader = Self {
            words,
            spelling,
            methods: Vec::new(),
        };
        let containers = reader.containers();
        reader.read_containers(&containers);
        reader.attach_nested(&containers);
        reader.methods
    }

    fn attach_nested(&mut self, containers: &[Container]) {
        for method in &mut self.methods {
            method.nested = Self::nested_in(containers, &method.body);
        }
    }

    fn nested_in(containers: &[Container], body: &Range<usize>) -> Vec<Range<usize>> {
        containers
            .iter()
            .filter(|container| {
                (container.range.start > body.start) & (container.range.end <= body.end)
            })
            .map(|container| container.range.start.saturating_sub(1)..container.range.end + 1)
            .collect()
    }

    fn containers(&self) -> Vec<Container> {
        if self.spelling.whole_file_is_container() {
            return vec![Container {
                range: 0..self.words.len(),
                depth: 0,
                public_members: false,
            }];
        }
        self.container_bodies()
    }

    fn container_bodies(&self) -> Vec<Container> {
        (0..self.words.len())
            .map(|at| self.container_at(at))
            .collect()
    }

    fn container_at(&self, at: usize) -> Container {
        let depth = self.words.depth(at);
        let open = self
            .is_container(at)
            .then(|| self.words.find_at_depth(at + 1, depth, &["{"]))
            .flatten();
        Container {
            range: self.words.inside_of(open),
            depth: depth + 1,
            public_members: self.has_public_members(at, open),
        }
    }

    fn has_public_members(&self, at: usize, open: Option<usize>) -> bool {
        let header = at + 1..open.unwrap_or(at);
        let trait_impl = (self.words.text(at) == "impl") & self.words.has_word(header, &["for"]);
        let line = self.words.line_begin(at)..at;
        let public_trait = (self.words.text(at) == "trait") & self.words.has_word(line, &["pub"]);
        trait_impl | public_trait
    }

    fn is_container(&self, at: usize) -> bool {
        let before = self.words.pair(at.wrapping_sub(2));
        let blocked = Self::CONTAINER_BLOCKERS.contains(&self.words.text(at.wrapping_sub(1)))
            | Self::CONTAINER_BLOCKERS.contains(&before);
        self.spelling.is_container(self.words.text(at)) & !blocked
    }

    fn read_containers(&mut self, containers: &[Container]) {
        for container in containers {
            self.read_members(container);
        }
    }

    fn read_members(&mut self, container: &Container) {
        let mut at = container.range.start;
        while at < container.range.end {
            at = self.read_member(at, container);
        }
    }

    fn read_member(&mut self, at: usize, container: &Container) -> usize {
        let head = self.head(at, container.depth);
        self.take(head, at, container.public_members)
    }

    fn head(&self, at: usize, depth: usize) -> Option<Head> {
        let starts = (self.words.depth(at) == depth) & self.starts_head(at);
        starts.then(|| self.head_parts(at, depth))
    }

    fn take(&mut self, head: Option<Head>, at: usize, public_members: bool) -> usize {
        let Some(head) = head else {
            return at + 1;
        };
        let method = self.method(&head, public_members);
        self.methods.push(method);
        head.resume
    }

    fn starts_head(&self, at: usize) -> bool {
        let keyword =
            self.spelling.is_head(self.words.text(at)) & !self.words.after_member_access(at);
        let named = !self.spelling.names_by_keyword() & self.named_head(at);
        keyword | named
    }

    fn named_head(&self, at: usize) -> bool {
        let before = self.words.text(at.wrapping_sub(1));
        let text = self.words.text(at);
        let allowed = !Self::NOT_BEFORE_NAME.contains(&before) & !Self::RESERVED.contains(&text);
        self.words.is_name(at) & (self.words.text(at + 1) == "(") & allowed
    }

    fn head_parts(&self, at: usize, depth: usize) -> Head {
        let name_at = self.name_index(at);
        let opener =
            self.words
                .find_at_depth(name_at + 1, depth, Self::body_openers(self.words.text(at)));
        let line_column = self.words.column(self.words.line_begin(at));
        let (body, resume) = self.body(opener, depth, line_column);
        Head {
            at,
            name_at,
            body,
            resume: resume.max(name_at + 1),
        }
    }

    fn body_openers(head: &str) -> &'static [&'static str] {
        match head {
            "fn" => Self::RUST_BODY_OPENERS,
            _ => Self::BODY_OPENERS,
        }
    }

    fn name_index(&self, at: usize) -> usize {
        let keyword =
            self.spelling.names_by_keyword() & !Spelling::is_self_named_head(self.words.text(at));
        let after_keyword = at + usize::from(keyword);
        let receiver =
            self.spelling.has_receiver_groups() & (self.words.text(after_keyword) == "(");
        if receiver {
            return self.words.after_group(after_keyword);
        }
        after_keyword
    }

    fn body(&self, opener: Option<usize>, depth: usize, column: usize) -> (Range<usize>, usize) {
        let at = opener.unwrap_or(usize::MAX);
        match self.words.text(at) {
            "{" => (self.words.inside(at), self.words.after_group(at)),
            "=" => {
                let end = self.words.expression_end(at + 1, depth, column);
                (at + 1..end, end)
            }
            _ => (0..0, 0),
        }
    }

    fn method(&self, head: &Head, public_members: bool) -> Method {
        Method {
            name: self.words.text(head.name_at).to_owned(),
            head: head.at,
            public: public_members | !self.is_private(head),
            body: head.body.clone(),
            receiver: self.receiver(head),
            nested: Vec::new(),
        }
    }

    fn receiver(&self, head: &Head) -> String {
        let named = self.spelling.has_receiver_groups() & (self.words.text(head.at + 1) == "(");
        if named {
            return self.words.text(head.at + 2).to_owned();
        }
        String::new()
    }

    fn is_private(&self, head: &Head) -> bool {
        let name = self.words.text(head.name_at);
        let underscore = name.starts_with('_');
        let hash = self.words.text(head.name_at.wrapping_sub(1)) == "#";
        let keyword = self.has_private_word(head);
        let lowercase = self.spelling.has_receiver_groups() & name.starts_with(char::is_lowercase);
        let unmarked = !self.spelling.public_words().is_empty() & !self.has_public_word(head);
        underscore | hash | keyword | lowercase | unmarked
    }

    fn has_private_word(&self, head: &Head) -> bool {
        let line = self.words.line_begin(head.at)..head.at;
        line.clone()
            .any(|at| self.spelling.is_private_word(self.words.text(at)))
    }

    fn has_public_word(&self, head: &Head) -> bool {
        let line = self.words.line_begin(head.at)..head.at;
        self.words.has_word(line, self.spelling.public_words())
    }
}
