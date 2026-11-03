use super::css_rule::CssRule;
use super::css_token::CssToken;

pub(super) struct CssRules;

impl CssRules {
    const MAX_DEPTH: usize = 32;

    pub(super) fn read<'a>(tokens: &'a [CssToken<'a>]) -> Result<Vec<CssRule<'a>>, &'static str> {
        let mut rules = Vec::new();
        Self::read_all(tokens, &mut rules)?;
        Ok(rules)
    }

    fn read_all<'a>(
        mut tokens: &'a [CssToken<'a>],
        rules: &mut Vec<CssRule<'a>>,
    ) -> Result<(), &'static str> {
        while !tokens.is_empty() {
            let (rule, used) = Self::next(tokens)?;
            rules.push(rule);
            tokens = &tokens[used..];
        }
        Ok(())
    }

    fn next<'a>(tokens: &'a [CssToken<'a>]) -> Result<(CssRule<'a>, usize), &'static str> {
        let boundary = Self::header_end(tokens)?;
        let body = Self::body_end(tokens, boundary)?;
        let rule = CssRule {
            header: &tokens[..boundary],
            body: body.map(|end| &tokens[boundary + 1..end]),
            start: tokens[0].start,
        };
        Ok((rule, body.unwrap_or(boundary) + 1))
    }

    fn header_end(tokens: &[CssToken<'_>]) -> Result<usize, &'static str> {
        let mut groups = Vec::new();
        let mut position = 0;
        while Self::header_next(tokens, position, &mut groups)? {
            position += 1;
        }
        Ok(position)
    }

    fn header_next(
        tokens: &[CssToken<'_>],
        position: usize,
        groups: &mut Vec<&'static str>,
    ) -> Result<bool, &'static str> {
        let token = tokens
            .get(position)
            .ok_or("Stylesheet rule has no block or semicolon.")?;
        let boundary = Self::header_step(token.text, groups)?;
        Ok(!boundary)
    }

    fn header_step(text: &str, groups: &mut Vec<&'static str>) -> Result<bool, &'static str> {
        match text {
            "(" if groups.len() < Self::MAX_DEPTH => {
                groups.push(")");
                Ok(false)
            }
            "[" if groups.len() < Self::MAX_DEPTH => {
                groups.push("]");
                Ok(false)
            }
            "(" | "[" => Err("Stylesheet group nesting exceeds 32 levels."),
            ")" | "]" => {
                Self::close_group(text, groups)?;
                Ok(false)
            }
            "{" | ";" if groups.is_empty() => Ok(true),
            "}" => Err("Stylesheet has an unexpected closing block."),
            _ => Ok(false),
        }
    }

    fn close_group(text: &str, groups: &mut Vec<&'static str>) -> Result<(), &'static str> {
        groups
            .pop()
            .filter(|expected| *expected == text)
            .ok_or("Stylesheet has mismatched parentheses or brackets.")
            .map(|_| ())
    }

    fn body_end(tokens: &[CssToken<'_>], boundary: usize) -> Result<Option<usize>, &'static str> {
        match tokens[boundary].text {
            ";" => Ok(None),
            _ => Self::block_end(tokens, boundary).map(Some),
        }
    }

    fn block_end(tokens: &[CssToken<'_>], boundary: usize) -> Result<usize, &'static str> {
        let mut depth = 1;
        let mut position = boundary + 1;
        while depth != 0 {
            let token = tokens
                .get(position)
                .ok_or("Stylesheet has an unclosed block.")?;
            depth = Self::depth_after(token.text, depth)?;
            position += 1;
        }
        Ok(position - 1)
    }

    fn depth_after(text: &str, depth: usize) -> Result<usize, &'static str> {
        match text {
            "{" if depth >= Self::MAX_DEPTH => Err("Stylesheet nesting exceeds 32 levels."),
            "{" => Ok(depth + 1),
            "}" => Ok(depth - 1),
            _ => Ok(depth),
        }
    }
}
