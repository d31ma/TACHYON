use super::SecretMatch;

#[derive(Clone, Copy)]
pub(super) struct SecretPattern {
    kind: &'static str,
    prefix: &'static str,
    accepts: fn(u8) -> bool,
    length: usize,
}

impl SecretPattern {
    const AWS: &'static str = "an AWS access key";
    const GITHUB: &'static str = "a GitHub token";
    const STRIPE: &'static str = "a Stripe secret key";
    const SLACK: &'static str = "a Slack token";
    const GOOGLE: &'static str = "a Google API key";
    const AZURE: &'static str = "an Azure storage key";
    const PRIVATE_KEY: &'static str = "a private key";
    const TOKEN: &'static str = "a JSON Web Token";
    const PRIVATE_KEY_END: &'static str = "PRIVATE KEY-----";
    const TOKEN_PARTS: usize = 3;
    const TOKEN_PART_LENGTH: usize = 10;
    const ALL: [Self; 16] = [
        Self::new(Self::AWS, "AKIA", Self::upper_or_digit, 16),
        Self::new(Self::AWS, "ASIA", Self::upper_or_digit, 16),
        Self::new(Self::GITHUB, "ghp_", Self::word, 36),
        Self::new(Self::GITHUB, "gho_", Self::word, 36),
        Self::new(Self::GITHUB, "ghu_", Self::word, 36),
        Self::new(Self::GITHUB, "ghs_", Self::word, 36),
        Self::new(Self::GITHUB, "github_pat_", Self::word, 22),
        Self::new(Self::STRIPE, "sk_live_", Self::word, 20),
        Self::new(Self::STRIPE, "rk_live_", Self::word, 20),
        Self::new(Self::SLACK, "xoxb-", Self::dashed, 20),
        Self::new(Self::SLACK, "xoxp-", Self::dashed, 20),
        Self::new(Self::SLACK, "xoxa-", Self::dashed, 20),
        Self::new(Self::SLACK, "hooks.slack.com/services/", Self::path, 20),
        Self::new(Self::GOOGLE, "AIza", Self::dashed, 35),
        Self::new(Self::AZURE, "AccountKey=", Self::base64, 40),
        Self::new(Self::PRIVATE_KEY, "-----BEGIN ", Self::key_header, 0),
    ];

    const fn new(
        kind: &'static str,
        prefix: &'static str,
        accepts: fn(u8) -> bool,
        length: usize,
    ) -> Self {
        Self {
            kind,
            prefix,
            accepts,
            length,
        }
    }

    pub(super) fn find(text: &str, line: usize) -> Vec<SecretMatch> {
        let mut found = Self::fixed_patterns(text, line);
        found.extend(Self::web_token(text, line));
        found
    }

    fn fixed_patterns(text: &str, line: usize) -> Vec<SecretMatch> {
        Self::ALL
            .iter()
            .filter_map(|pattern| pattern.find_in(text, line))
            .collect()
    }

    fn find_in(&self, text: &str, line: usize) -> Option<SecretMatch> {
        let start = text.find(self.prefix)?;
        let value = self.value_at(text, start);
        self.is_long_enough(&value, text).then_some(SecretMatch {
            line,
            kind: self.kind,
            value,
        })
    }

    fn value_at(&self, text: &str, start: usize) -> String {
        let rest = &text[start + self.prefix.len()..];
        let length = rest
            .bytes()
            .take_while(|byte| (self.accepts)(*byte))
            .count();
        String::from(&text[start..start + self.prefix.len() + length])
    }

    fn is_long_enough(&self, value: &str, text: &str) -> bool {
        if self.kind == Self::PRIVATE_KEY {
            return text.contains(Self::PRIVATE_KEY_END);
        }
        value.len() >= self.prefix.len() + self.length
    }

    fn web_token(text: &str, line: usize) -> Option<SecretMatch> {
        let start = text.find("eyJ")?;
        let value: String = text[start..]
            .chars()
            .take_while(|character| Self::token_character(*character))
            .collect();
        Self::is_web_token(&value).then_some(SecretMatch {
            line,
            kind: Self::TOKEN,
            value,
        })
    }

    fn is_web_token(value: &str) -> bool {
        let parts: Vec<&str> = value.split('.').collect();
        let long_parts = parts
            .iter()
            .filter(|part| part.len() >= Self::TOKEN_PART_LENGTH)
            .count();
        [parts.len(), long_parts] == [Self::TOKEN_PARTS, Self::TOKEN_PARTS]
    }

    fn token_character(character: char) -> bool {
        matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.')
    }

    fn upper_or_digit(byte: u8) -> bool {
        byte.is_ascii_uppercase() || byte.is_ascii_digit()
    }

    fn word(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }

    fn dashed(byte: u8) -> bool {
        matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_')
    }

    fn path(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'/'
    }

    fn base64(byte: u8) -> bool {
        matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'+' | b'/' | b'=')
    }

    fn key_header(byte: u8) -> bool {
        byte.is_ascii_uppercase() || byte == b' '
    }
}
