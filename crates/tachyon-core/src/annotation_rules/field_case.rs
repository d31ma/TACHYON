use super::scan::Messages;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FieldCase {
    LowerCamel,
    LowerCamelOrPrivate,
    Snake,
    SnakeOrPrivate,
    PrivateCamel,
    Pascal,
}

impl FieldCase {
    const LANGUAGES: [(Self, &'static [&'static str]); 5] = [
        (
            Self::LowerCamel,
            &["js", "ts", "java", "groovy", "kt", "swift", "go", "php"],
        ),
        (Self::LowerCamelOrPrivate, &["dart"]),
        (Self::Snake, &["rs", "rb"]),
        (Self::SnakeOrPrivate, &["py"]),
        (Self::PrivateCamel, &["cs"]),
    ];
    const PUBLIC_FORMS: [(Self, Self); 1] = [(Self::PrivateCamel, Self::Pascal)];
    const PRIVATE_MARK: &'static str = "_";
    const SNAKE_LETTERS: &'static str = "abcdefghijklmnopqrstuvwxyz0123456789_";

    pub(crate) fn of(language: &str, public_member: bool) -> Option<Self> {
        Self::LANGUAGES
            .iter()
            .find(|(_, languages)| languages.contains(&language))
            .map(|(case, _)| case.for_member(public_member))
    }

    pub(crate) fn accepts(self, name: &str) -> bool {
        self.checked(name)
    }

    pub(crate) fn spelled(self, name: &str) -> String {
        self.spelled_words(name, &Self::words(name))
    }

    pub(crate) fn described(self) -> &'static str {
        self.description()
    }

    fn for_member(self, public_member: bool) -> Self {
        Self::PUBLIC_FORMS
            .iter()
            .find(|(case, _)| (*case, true) == (self, public_member))
            .map_or(self, |(_, public_case)| *public_case)
    }

    fn checked(self, name: &str) -> bool {
        match self {
            Self::LowerCamel => Self::is_lower_camel(name),
            Self::LowerCamelOrPrivate => Self::is_lower_camel(Self::without_mark(name)),
            Self::Snake => Self::is_snake(name),
            Self::SnakeOrPrivate => Self::is_snake(Self::without_mark(name)),
            Self::PrivateCamel => Self::is_marked_camel(name),
            Self::Pascal => Self::is_pascal(name),
        }
    }

    fn spelled_words(self, name: &str, words: &[String]) -> String {
        match self {
            Self::LowerCamel => Self::camel(words),
            Self::LowerCamelOrPrivate => Self::kept_mark(name) + &Self::camel(words),
            Self::Snake => words.join("_"),
            Self::SnakeOrPrivate => Self::kept_mark(name) + &words.join("_"),
            Self::PrivateCamel => Self::PRIVATE_MARK.to_owned() + &Self::camel(words),
            Self::Pascal => Self::pascal(words),
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::LowerCamel | Self::LowerCamelOrPrivate => "lowerCamel case, like cartStore",
            Self::Snake | Self::SnakeOrPrivate => "snake_case, like cart_store",
            Self::PrivateCamel => {
                "an underscore and lowerCamel case for a private field, like _cartStore"
            }
            Self::Pascal => {
                "PascalCase for a property or a public or protected field, like CartStore"
            }
        }
    }

    fn without_mark(name: &str) -> &str {
        name.strip_prefix(Self::PRIVATE_MARK).unwrap_or(name)
    }

    fn kept_mark(name: &str) -> String {
        let marked = name.starts_with(Self::PRIVATE_MARK);
        Self::PRIVATE_MARK.repeat(usize::from(marked))
    }

    fn is_marked_camel(name: &str) -> bool {
        name.strip_prefix(Self::PRIVATE_MARK)
            .is_some_and(Self::is_lower_camel)
    }

    fn is_lower_camel(name: &str) -> bool {
        let first = name.starts_with(|letter: char| letter.is_ascii_lowercase());
        [first, Self::is_alphanumeric(name)] == [true, true]
    }

    fn is_pascal(name: &str) -> bool {
        let first = name.starts_with(|letter: char| letter.is_ascii_uppercase());
        [first, Self::is_alphanumeric(name)] == [true, true]
    }

    fn is_snake(name: &str) -> bool {
        let first = name.starts_with(|letter: char| letter.is_ascii_lowercase());
        let rest = name
            .chars()
            .all(|letter| Self::SNAKE_LETTERS.contains(letter));
        [first, rest] == [true, true]
    }

    fn is_alphanumeric(name: &str) -> bool {
        name.chars().all(|letter| letter.is_ascii_alphanumeric())
    }

    fn words(name: &str) -> Vec<String> {
        Messages::snake_case(name)
            .split('_')
            .filter(|word| !word.is_empty())
            .map(str::to_owned)
            .collect()
    }

    fn camel(words: &[String]) -> String {
        words
            .split_first()
            .map_or_else(String::new, |(first, rest)| {
                first.clone() + &Self::pascal(rest)
            })
    }

    fn pascal(words: &[String]) -> String {
        words.iter().map(|word| Self::capitalized(word)).collect()
    }

    fn capitalized(word: &str) -> String {
        let mut letters = word.chars();
        letters.next().map_or_else(String::new, |first| {
            first.to_ascii_uppercase().to_string() + letters.as_str()
        })
    }
}
