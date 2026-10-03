use super::HostLabel;

#[derive(Clone, Copy, Debug)]
pub struct LocalHostName;

impl LocalHostName {
    const NAME: &'static str = "localhost";
    const SUFFIX: &'static str = ".localhost";
    const TRAILING_DOT: char = '.';
    const MAX_NAME_LENGTH: usize = 253;

    #[must_use]
    pub fn is_local(host: &str) -> bool {
        let name = Self::plain(host);
        [name == Self::NAME, name.ends_with(Self::SUFFIX)].contains(&true)
    }

    #[must_use]
    pub fn bind_name(text: &str) -> Option<String> {
        let name = Self::plain(text);
        let short_enough = name.len() <= Self::MAX_NAME_LENGTH;
        let checks = [
            Self::is_local(&name),
            short_enough,
            HostLabel::is_name(&name),
        ];
        (checks == [true, true, true]).then_some(name)
    }

    fn plain(text: &str) -> String {
        text.strip_suffix(Self::TRAILING_DOT)
            .unwrap_or(text)
            .to_ascii_lowercase()
    }
}
