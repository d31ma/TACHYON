pub(super) struct PlaceholderFill;

impl PlaceholderFill {
    pub(super) fn once(template: &str, values: &[(&str, &str)]) -> String {
        let mut filled = String::with_capacity(template.len());
        let rest = Self::fill(template, values, &mut filled);
        filled.push_str(rest);
        filled
    }

    fn fill<'t>(template: &'t str, values: &[(&str, &str)], filled: &mut String) -> &'t str {
        let mut rest = template;
        while let Some((at, placeholder, value)) = Self::next(rest, values) {
            filled.push_str(&rest[..at]);
            filled.push_str(value);
            rest = &rest[at + placeholder.len()..];
        }
        rest
    }

    fn next<'v>(text: &str, values: &[(&'v str, &'v str)]) -> Option<(usize, &'v str, &'v str)> {
        values
            .iter()
            .filter_map(|(placeholder, value)| {
                text.find(placeholder).map(|at| (at, *placeholder, *value))
            })
            .min_by_key(|(at, _, _)| *at)
    }
}
