use super::super::class_file::ClassFile;
use super::super::design_problem::DesignProblem;
use super::super::design_rule::DesignRule;
use super::known_classes::KnownClasses;
use std::ops::Range;

pub(super) struct YardNames;

impl YardNames {
    const RUBY: &'static str = "rb";
    const TAGS: [&'static str; 2] = ["@param", "@return"];
    const OPEN: char = '[';
    const CLOSE: char = ']';
    const NEWLINE: char = '\n';
    const QUALIFIER: &'static str = "::";
    const NAME_MARKS: &'static str = "_:";
    const PLAIN_WORDS: [&'static str; 5] = ["nil", "true", "false", "void", "self"];
    const RUBY_CLASSES: [&'static str; 42] = [
        "String",
        "Integer",
        "Float",
        "Numeric",
        "Array",
        "Hash",
        "Symbol",
        "Boolean",
        "TrueClass",
        "FalseClass",
        "NilClass",
        "Object",
        "BasicObject",
        "Time",
        "Date",
        "DateTime",
        "Proc",
        "Range",
        "Regexp",
        "IO",
        "File",
        "Struct",
        "Set",
        "Exception",
        "StandardError",
        "ArgumentError",
        "RuntimeError",
        "BigDecimal",
        "Rational",
        "Complex",
        "Enumerator",
        "Method",
        "Class",
        "Module",
        "Comparable",
        "Enumerable",
        "Thread",
        "Queue",
        "Mutex",
        "Encoding",
        "SecureRandom",
        "Data",
    ];
    const FRAMEWORK_CLASSES: [&'static str; 14] = [
        "ApplicationEventPublisher",
        "DataSource",
        "RedisTemplate",
        "SocketSession",
        "SocketPeer",
        "Clipboard",
        "Clock",
        "Connectivity",
        "Geolocation",
        "Notifications",
        "PageVisibility",
        "Preferences",
        "Share",
        "TacStorage",
    ];

    pub(super) fn problems(class: &ClassFile, known: &KnownClasses) -> Vec<DesignProblem> {
        let ruby = class.extension == Self::RUBY;
        Self::type_words(class.text)
            .into_iter()
            .filter(|_| ruby)
            .filter(|(_, word)| !Self::is_known(word, known))
            .map(|(at, word)| Self::problem(class, at, word))
            .collect()
    }

    fn type_words(text: &str) -> Vec<(usize, &str)> {
        Self::TAGS
            .iter()
            .flat_map(|tag| text.match_indices(tag))
            .filter_map(|(at, tag)| Self::bracket(text, at + tag.len()))
            .flat_map(|range| Self::words_in(text, range))
            .collect()
    }

    fn bracket(text: &str, from: usize) -> Option<Range<usize>> {
        let line_end = text[from..]
            .find(Self::NEWLINE)
            .map_or(text.len(), |newline| from + newline);
        let line = &text[from..line_end];
        let open = line.find(Self::OPEN)?;
        let close = line.rfind(Self::CLOSE)?;
        (open < close).then(|| from + open + 1..from + close)
    }

    fn words_in(text: &str, range: Range<usize>) -> Vec<(usize, &str)> {
        text[range.clone()]
            .split(Self::is_separator)
            .scan(range.start, |offset, word| {
                let at = *offset;
                *offset += word.len() + 1;
                Some((at, word))
            })
            .filter(|(_, word)| !word.is_empty())
            .collect()
    }

    fn is_separator(character: char) -> bool {
        !(Self::NAME_MARKS.contains(character) | character.is_ascii_alphanumeric())
    }

    fn is_known(word: &str, known: &KnownClasses) -> bool {
        let qualified = word.contains(Self::QUALIFIER);
        let plain = Self::PLAIN_WORDS.contains(&word);
        let ruby = Self::RUBY_CLASSES.contains(&word);
        let framework = Self::FRAMEWORK_CLASSES.contains(&word);
        qualified | plain | ruby | framework | known.contains(word)
    }

    fn problem(class: &ClassFile, at: usize, word: &str) -> DesignProblem {
        DesignProblem {
            rule: DesignRule::Types,
            path: class.path.to_owned(),
            start: at,
            end: at + word.len(),
            line: class.line(at),
            message: format!(
                "The type `{word}` in a comment of {} names no known class.",
                class.name
            ),
            help: String::from(
                "Name a Ruby class, a class or model of this project, a Tachyon class, or `Boolean`, `nil` or `void`. A type comment is only worth what it names (ADR 0079).",
            ),
        }
    }
}
