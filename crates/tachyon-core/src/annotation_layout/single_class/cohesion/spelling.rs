use super::layout::Layout;

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug)]
pub(in super::super) struct Spelling {
    keywords: &'static [&'static str],
    line_start_keywords: &'static [&'static str],
    pairs: &'static [&'static str],
    ternary: bool,
    lambda_marks: &'static [&'static str],
    trailing_blocks: &'static [&'static str],
    heads: &'static [&'static str],
    containers: &'static [&'static str],
    private_words: &'static [&'static str],
    query_syntax: bool,
    receiver_groups: bool,
    pub_methods: bool,
    parentheses_call: bool,
    layout: Layout,
    private_hint: &'static str,
}

impl Spelling {
    const UNCOUNTED: &'static [&'static str] = &["try", "finally", "begin", "ensure"];
    const SELF_NAMED_HEADS: &'static [&'static str] = &["init", "constructor"];
    const C_KEYWORDS: &'static [&'static str] =
        &["if", "switch", "for", "while", "try", "catch", "finally"];
    const CSHARP_KEYWORDS: &'static [&'static str] = &[
        "if", "switch", "for", "foreach", "while", "try", "catch", "finally",
    ];
    const KOTLIN_KEYWORDS: &'static [&'static str] =
        &["if", "when", "for", "while", "try", "catch", "finally"];
    const SWIFT_KEYWORDS: &'static [&'static str] =
        &["if", "guard", "switch", "for", "while", "catch"];
    const GO_KEYWORDS: &'static [&'static str] = &["if", "switch", "select", "for"];
    const RUST_KEYWORDS: &'static [&'static str] = &["if", "match", "for", "while", "loop"];
    const PHP_KEYWORDS: &'static [&'static str] = &[
        "if", "elseif", "switch", "match", "for", "foreach", "while", "try", "catch", "finally",
        "and", "or", "xor",
    ];
    const PYTHON_KEYWORDS: &'static [&'static str] = &[
        "if", "elif", "for", "while", "try", "except", "finally", "and", "or", "match",
    ];
    const RUBY_KEYWORDS: &'static [&'static str] = &[
        "if", "unless", "elsif", "case", "while", "until", "for", "loop", "begin", "rescue",
        "ensure", "and", "or",
    ];
    const LOGICAL_PAIRS: &'static [&'static str] = &["&&", "||"];
    const NULLISH_PAIRS: &'static [&'static str] = &["&&", "||", "??"];
    const KOTLIN_PAIRS: &'static [&'static str] = &["&&", "||", "?:"];
    const PHP_PAIRS: &'static [&'static str] = &["&&", "||", "??", "?:"];
    const ITERATORS: &'static [&'static str] = &[
        "forEach",
        "map",
        "filter",
        "reduce",
        "reduceRight",
        "flatMap",
        "some",
        "every",
        "find",
        "findIndex",
        "findLast",
        "findLastIndex",
        "sort",
        "toSorted",
        "each",
        "each_with_index",
        "each_with_object",
        "each_pair",
        "each_key",
        "each_value",
        "each_slice",
        "collect",
        "select",
        "reject",
        "detect",
        "inject",
        "sum",
        "count",
        "times",
        "upto",
        "downto",
        "step",
        "any",
        "all",
        "none",
        "filter_map",
        "flat_map",
        "sort_by",
        "group_by",
        "partition",
        "min_by",
        "max_by",
        "forEachIndexed",
        "mapNotNull",
        "mapIndexed",
        "filterNot",
        "fold",
        "foldRight",
        "associate",
        "associateBy",
        "associateWith",
        "groupBy",
        "sortedBy",
        "sortedByDescending",
        "sortedWith",
        "sumOf",
        "first",
        "firstOrNull",
        "last",
        "lastOrNull",
        "repeat",
        "compactMap",
        "allSatisfy",
        "where",
        "Where",
        "Select",
        "SelectMany",
        "Any",
        "All",
        "First",
        "FirstOrDefault",
        "Last",
        "LastOrDefault",
        "Single",
        "OrderBy",
        "OrderByDescending",
        "ThenBy",
        "GroupBy",
        "Aggregate",
        "Sum",
        "Count",
        "ForEach",
        "TakeWhile",
        "SkipWhile",
        "Min",
        "Max",
        "for_each",
        "find_map",
        "take_while",
        "skip_while",
        "position",
        "retain",
        "sort_by_key",
        "max_by_key",
        "min_by_key",
        "array_map",
        "array_filter",
        "array_reduce",
        "array_walk",
        "usort",
        "uasort",
        "uksort",
        "removeWhere",
        "retainWhere",
        "indexWhere",
        "firstWhere",
        "lastWhere",
        "expand",
        "takeWhile",
        "skipWhile",
        "anyMatch",
        "allMatch",
        "noneMatch",
        "peek",
        "removeIf",
        "replaceAll",
        "sorted",
        "min",
        "max",
        "IndexFunc",
        "ContainsFunc",
        "SortFunc",
        "SortStableFunc",
        "DeleteFunc",
        "Slice",
        "SliceStable",
        "contains",
        "filterIndexed",
        "sortBy",
        "sortWith",
    ];

    const JAVASCRIPT: Self = Self {
        keywords: Self::C_KEYWORDS,
        line_start_keywords: &[],
        pairs: Self::NULLISH_PAIRS,
        ternary: true,
        lambda_marks: &["=>", "function"],
        trailing_blocks: &[],
        heads: &[],
        containers: &["class"],
        private_words: &[],
        query_syntax: false,
        receiver_groups: false,
        pub_methods: false,
        parentheses_call: true,
        layout: Layout::Braced,
        private_hint: "a private method whose name starts with `#`, such as `#pick(value)`",
    };

    const TYPESCRIPT: Self = Self {
        private_words: &["private"],
        private_hint: "a private method, such as `private pick(value: Item): Item`",
        ..Self::JAVASCRIPT
    };

    const JAVA: Self = Self {
        pairs: Self::LOGICAL_PAIRS,
        lambda_marks: &["->"],
        containers: &["class", "record"],
        private_words: &["private"],
        private_hint: "a private method, such as `private Item pick(Item value)`",
        ..Self::JAVASCRIPT
    };

    const CSHARP: Self = Self {
        keywords: Self::CSHARP_KEYWORDS,
        lambda_marks: &["=>", "delegate"],
        containers: &["class", "record", "struct"],
        private_words: &["private"],
        query_syntax: true,
        private_hint: "a private method, such as `private Item Pick(Item value)`",
        ..Self::JAVASCRIPT
    };

    const DART: Self = Self {
        lambda_marks: &["=>", "{"],
        containers: &["class", "mixin", "extension"],
        private_hint: "a private method whose name starts with `_`, such as `Item _pick(Item value)`",
        ..Self::JAVASCRIPT
    };

    const KOTLIN: Self = Self {
        keywords: Self::KOTLIN_KEYWORDS,
        pairs: Self::KOTLIN_PAIRS,
        ternary: false,
        lambda_marks: &["{"],
        trailing_blocks: &["{"],
        heads: &["fun", "constructor", "init"],
        containers: &["class", "object"],
        private_words: &["private"],
        private_hint: "a private method, such as `private fun pick(value: Item): Item`",
        ..Self::JAVASCRIPT
    };

    const SWIFT: Self = Self {
        keywords: Self::SWIFT_KEYWORDS,
        lambda_marks: &["{"],
        trailing_blocks: &["{"],
        heads: &["func", "init"],
        containers: &["class", "struct", "extension", "actor"],
        private_words: &["private", "fileprivate"],
        private_hint: "a private method, such as `private func pick(_ value: Item) -> Item`",
        ..Self::JAVASCRIPT
    };

    const PHP: Self = Self {
        keywords: Self::PHP_KEYWORDS,
        pairs: Self::PHP_PAIRS,
        lambda_marks: &["fn", "function"],
        heads: &["function"],
        private_words: &["private"],
        private_hint: "a private method, such as `private function pick(Item $value): Item`",
        ..Self::JAVASCRIPT
    };

    const RUST: Self = Self {
        keywords: Self::RUST_KEYWORDS,
        pairs: Self::LOGICAL_PAIRS,
        ternary: false,
        lambda_marks: &["|"],
        heads: &["fn"],
        containers: &["impl", "trait"],
        pub_methods: true,
        private_hint: "a private function without `pub`, such as `fn pick(&self, value: &Item) -> Item`",
        ..Self::JAVASCRIPT
    };

    const GO: Self = Self {
        keywords: Self::GO_KEYWORDS,
        pairs: Self::LOGICAL_PAIRS,
        ternary: false,
        lambda_marks: &["func"],
        heads: &["func"],
        containers: &[],
        receiver_groups: true,
        private_hint: "a method with a lowercase name, such as `func (service *Service) pick(value Item) Item`",
        ..Self::JAVASCRIPT
    };

    const PYTHON: Self = Self {
        keywords: Self::PYTHON_KEYWORDS,
        line_start_keywords: &["match"],
        pairs: &[],
        ternary: false,
        lambda_marks: &["lambda"],
        containers: &[],
        layout: Layout::Indented,
        private_hint: "a private method whose name starts with `_`, such as `def _pick(self, value)`",
        ..Self::JAVASCRIPT
    };

    const RUBY: Self = Self {
        keywords: Self::RUBY_KEYWORDS,
        pairs: Self::LOGICAL_PAIRS,
        lambda_marks: &[],
        trailing_blocks: &["{", "do"],
        containers: &[],
        parentheses_call: false,
        layout: Layout::Ended,
        private_hint: "a method below `private`, such as `def pick(value)`",
        ..Self::JAVASCRIPT
    };

    const LANGUAGES: &'static [(&'static str, Self)] = &[
        ("js", Self::JAVASCRIPT),
        ("ts", Self::TYPESCRIPT),
        ("java", Self::JAVA),
        ("cs", Self::CSHARP),
        ("dart", Self::DART),
        ("kt", Self::KOTLIN),
        ("swift", Self::SWIFT),
        ("php", Self::PHP),
        ("rs", Self::RUST),
        ("go", Self::GO),
        ("py", Self::PYTHON),
        ("rb", Self::RUBY),
    ];

    pub(in super::super) fn of(extension: &str) -> Option<Self> {
        Self::LANGUAGES
            .iter()
            .find(|(name, _)| *name == extension)
            .map(|(_, spelling)| *spelling)
    }

    pub(in super::super) fn is_keyword(&self, word: &str) -> bool {
        self.keywords.contains(&word)
    }

    pub(in super::super) fn needs_line_start(&self, word: &str) -> bool {
        self.line_start_keywords.contains(&word)
    }

    pub(in super::super) fn is_pair(&self, pair: &str) -> bool {
        self.pairs.contains(&pair)
    }

    pub(in super::super) const fn has_ternary(&self) -> bool {
        self.ternary
    }

    pub(in super::super) fn is_iterator(word: &str) -> bool {
        Self::ITERATORS.contains(&word)
    }

    pub(in super::super) fn is_lambda_mark(&self, word: &str) -> bool {
        self.lambda_marks.contains(&word)
    }

    pub(in super::super) fn opens_trailing_block(&self, word: &str) -> bool {
        self.trailing_blocks.contains(&word)
    }

    pub(in super::super) fn is_head(&self, word: &str) -> bool {
        self.heads.contains(&word)
    }

    pub(in super::super) fn names_by_keyword(&self) -> bool {
        !self.heads.is_empty()
    }

    pub(in super::super) fn is_self_named_head(word: &str) -> bool {
        Self::SELF_NAMED_HEADS.contains(&word)
    }

    pub(in super::super) fn is_container(&self, word: &str) -> bool {
        self.containers.contains(&word)
    }

    pub(in super::super) fn whole_file_is_container(&self) -> bool {
        self.containers.is_empty()
    }

    pub(in super::super) fn is_private_word(&self, word: &str) -> bool {
        self.private_words.contains(&word)
    }

    pub(in super::super) const fn has_query_syntax(&self) -> bool {
        self.query_syntax
    }

    pub(in super::super) const fn has_receiver_groups(&self) -> bool {
        self.receiver_groups
    }

    pub(in super::super) const fn public_by_pub(&self) -> bool {
        self.pub_methods
    }

    pub(in super::super) const fn calls_need_parentheses(&self) -> bool {
        self.parentheses_call
    }

    pub(in super::super) fn groups_conditions(&self) -> bool {
        self.layout == Layout::Braced
    }

    pub(in super::super) const fn layout(&self) -> Layout {
        self.layout
    }

    pub(in super::super) const fn private_hint(&self) -> &'static str {
        self.private_hint
    }

    pub(in super::super) fn counts(word: &str) -> bool {
        !Self::UNCOUNTED.contains(&word)
    }
}
