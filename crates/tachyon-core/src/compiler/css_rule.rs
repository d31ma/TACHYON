use super::css_token::CssToken;

pub(super) struct CssRule<'a> {
    pub(super) header: &'a [CssToken<'a>],
    pub(super) body: Option<&'a [CssToken<'a>]>,
    pub(super) start: usize,
}
