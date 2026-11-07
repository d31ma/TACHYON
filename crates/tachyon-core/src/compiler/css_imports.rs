use super::css_rule::CssRule;
use super::css_rules::CssRules;
use super::css_token::CssToken;

pub(super) struct CssImports;

impl CssImports {
    pub(super) fn read(tokens: &[CssToken<'_>]) -> Result<Vec<String>, &'static str> {
        let rules = CssRules::read(tokens)?;
        let mut imports = Vec::new();
        rules
            .iter()
            .try_for_each(|rule| Self::add_rule(rule, &mut imports))?;
        Ok(imports)
    }

    fn add_rule(rule: &CssRule<'_>, imports: &mut Vec<String>) -> Result<(), &'static str> {
        let first = rule.header.first().map_or("", |token| token.text);
        match first {
            "@import" => {
                imports.push(Self::specifier(rule)?);
                Ok(())
            }
            name if name.starts_with('@') & name.contains('\\') => {
                Err("Escaped CSS at-rule names are unsupported in the bounded import graph.")
            }
            _ => Self::check_nested(rule.body),
        }
    }

    fn check_nested(body: Option<&[CssToken<'_>]>) -> Result<(), &'static str> {
        let nested_import = body.into_iter().flatten().any(Self::nested_import);
        (!nested_import)
            .then_some(())
            .ok_or("Stylesheet imports must be ordinary top-level @import rules.")
    }

    fn nested_import(token: &CssToken<'_>) -> bool {
        token.text.starts_with("@import")
            | (token.text.starts_with('@') & token.text.contains('\\'))
    }

    fn specifier(rule: &CssRule<'_>) -> Result<String, &'static str> {
        let tokens = rule.header;
        rule.body
            .is_none()
            .then_some(())
            .ok_or("CSS @import must end with a semicolon.")?;
        let token = tokens
            .get(1)
            .ok_or("CSS @import needs a local quoted path.")?;
        let source = Self::source_token(tokens, token.text)?;
        Self::unquote(source)
    }

    fn source_token<'a>(
        tokens: &'a [CssToken<'a>],
        first: &'a str,
    ) -> Result<&'a str, &'static str> {
        match first {
            "url" if tokens.get(2).map(|token| token.text) == Some("(") => Self::url_token(tokens),
            _ => Ok(first),
        }
    }

    fn url_token<'a>(tokens: &'a [CssToken<'a>]) -> Result<&'a str, &'static str> {
        (tokens.get(4).map(|token| token.text) == Some(")"))
            .then_some(())
            .ok_or("CSS import url() requires one ordinary quoted local path.")?;
        tokens
            .get(3)
            .map(|token| token.text)
            .ok_or("CSS import URL is empty.")
    }

    fn unquote(source: &str) -> Result<String, &'static str> {
        let quoted = source.starts_with(['\'', '"'])
            & (source.as_bytes().first() == source.as_bytes().last());
        (quoted & (source.len() >= 2) & !source.contains('\\'))
            .then_some(())
            .ok_or("CSS imports require unescaped quoted local paths.")?;
        Ok(String::from(&source[1..source.len() - 1]))
    }
}
