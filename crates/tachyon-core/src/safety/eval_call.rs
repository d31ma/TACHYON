#[derive(Clone, Copy)]
pub(super) struct EvalCall {
    pub(super) word: &'static str,
    ruby: bool,
    opens_call: bool,
}

impl EvalCall {
    const RUBY: &'static str = "rb";
    const RUBY_BLOCK_STARTS: [&'static str; 3] = ["{", "do", "(&"];
    const WORD_SEPARATOR: char = ',';
    const TYPE_NAMES: [&'static str; 3] = [
        "ScriptEngineManager",
        "CSharpScript",
        "FsiEvaluationSession",
    ];
    const LANGUAGES: [(&'static str, &'static str); 12] = [
        ("js", "eval,new Function"),
        ("mjs", "eval,new Function"),
        ("ts", "eval,new Function"),
        ("py", "eval,exec"),
        ("rb", "eval,instance_eval,class_eval,module_eval"),
        ("php", "eval,create_function"),
        ("java", "ScriptEngineManager,.eval"),
        ("kt", "ScriptEngineManager,.eval,evaluateScript"),
        ("cs", "CSharpScript"),
        ("fs", "CSharpScript,FsiEvaluationSession"),
        ("swift", "evaluateScript"),
        ("dart", "evaluateJavascript"),
    ];

    pub(super) fn for_extension(extension: &str) -> Vec<Self> {
        let words = Self::words_of(extension);
        words
            .split(Self::WORD_SEPARATOR)
            .map(|word| Self::of(extension, word))
            .collect()
    }

    fn words_of(extension: &str) -> &'static str {
        Self::LANGUAGES
            .iter()
            .find(|(language, _)| *language == extension)
            .map_or("", |(_, words)| words)
    }

    fn of(extension: &str, word: &'static str) -> Self {
        let ruby = extension == Self::RUBY;
        let bare = [ruby, Self::TYPE_NAMES.contains(&word)];
        Self {
            word,
            ruby,
            opens_call: !bare.contains(&true),
        }
    }

    pub(super) fn found_in(&self, code: &str) -> Option<usize> {
        self.first_use(code)
    }

    fn first_use(&self, code: &str) -> Option<usize> {
        let (at, _) = code
            .match_indices(self.word)
            .find(|(at, _)| self.is_use(code, *at))?;
        Some(at)
    }

    fn is_use(&self, code: &str, at: usize) -> bool {
        let rest = code[at + self.word.len()..].trim_start();
        let shape = [
            self.starts_cleanly(code, at),
            Self::ends_word(&code[at + self.word.len()..]),
            self.opens_where_needed(rest),
            self.not_a_ruby_block(rest),
        ];
        !shape.contains(&false)
    }

    fn starts_cleanly(&self, code: &str, at: usize) -> bool {
        if self.word.starts_with('.') {
            return true;
        }
        Self::stands_alone(code, at)
    }

    fn stands_alone(code: &str, at: usize) -> bool {
        let before = code[..at].chars().next_back();
        !before.is_some_and(Self::joins_name)
    }

    fn joins_name(character: char) -> bool {
        matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$' | '.')
    }

    fn ends_word(after: &str) -> bool {
        !after.chars().next().is_some_and(Self::name_character)
    }

    fn opens_where_needed(&self, rest: &str) -> bool {
        !self.opens_call || rest.starts_with('(')
    }

    fn not_a_ruby_block(&self, rest: &str) -> bool {
        [!self.ruby, !Self::starts_block(rest)].contains(&true)
    }

    fn starts_block(rest: &str) -> bool {
        Self::RUBY_BLOCK_STARTS
            .iter()
            .any(|start| rest.starts_with(start))
    }

    fn name_character(character: char) -> bool {
        matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '$')
    }
}
