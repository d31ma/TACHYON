use super::css_token::CssToken;

pub(super) struct FoundationValues;

impl FoundationValues {
    const URL_PREFIX: &'static str = "url(";
    const FUNCTIONS: [&'static str; 2] = ["url", "expression"];

    pub(super) fn validate(tokens: &[CssToken<'_>]) -> Result<(), &'static str> {
        let syntax = tokens.iter().any(Self::unsafe_token);
        let function = tokens.windows(2).any(Self::unsafe_function);
        let code: String = tokens
            .iter()
            .filter(|token| !Self::literal(token))
            .map(|token| token.text)
            .collect();
        let important = code.to_ascii_lowercase().contains("!important");
        (!(syntax | function | important)).then_some(()).ok_or("Foundation values cannot load assets, execute expressions, escape syntax, insert nested rules or force !important overrides.")
    }

    fn unsafe_token(token: &CssToken<'_>) -> bool {
        match Self::literal(token) {
            true => false,
            false => Self::unquoted_token(token),
        }
    }

    fn unquoted_token(token: &CssToken<'_>) -> bool {
        (token.text == "{")
            | (token.text == "}")
            | token
                .text
                .to_ascii_lowercase()
                .starts_with(Self::URL_PREFIX)
            | token.text.starts_with('@')
            | token.text.contains('\\')
    }

    fn literal(token: &CssToken<'_>) -> bool {
        token.text.starts_with(['\'', '"'])
    }

    fn unsafe_function(tokens: &[CssToken<'_>]) -> bool {
        let name = tokens[0].text.to_ascii_lowercase();
        Self::FUNCTIONS.contains(&name.as_str()) & (tokens[1].text == "(")
    }
}
