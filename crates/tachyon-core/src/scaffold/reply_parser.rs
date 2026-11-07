use super::Choice;

pub(crate) struct ReplyParser;

impl ReplyParser {
    const SEPARATORS: [char; 2] = [',', ' '];

    pub(crate) fn many<T: Choice>(reply: &str, options: &[T], defaults: &[T]) -> Option<Vec<T>> {
        let parts = Self::parts(reply);
        let chosen = Self::chosen_or_defaults(&parts, options, defaults)?;
        Some(Self::in_list_order(&chosen, options))
    }

    pub(crate) fn one<T: Choice>(reply: &str, options: &[T], default: T) -> Option<T> {
        let chosen = Self::many(reply, options, &[default])?;
        Self::single(&chosen)
    }

    fn parts(reply: &str) -> Vec<&str> {
        reply
            .split(Self::SEPARATORS)
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect()
    }

    fn chosen_or_defaults<T: Choice>(
        parts: &[&str],
        options: &[T],
        defaults: &[T],
    ) -> Option<Vec<T>> {
        if parts.is_empty() {
            return Some(defaults.to_vec());
        }
        parts
            .iter()
            .map(|part| Self::choice(part, options))
            .collect()
    }

    fn choice<T: Choice>(part: &str, options: &[T]) -> Option<T> {
        let chosen = Self::numbered(part, options).or_else(|| T::named(part))?;
        options.contains(&chosen).then_some(chosen)
    }

    fn numbered<T: Choice>(part: &str, options: &[T]) -> Option<T> {
        let number: usize = part.parse().ok()?;
        options.get(number.checked_sub(1)?).copied()
    }

    fn in_list_order<T: Choice>(chosen: &[T], options: &[T]) -> Vec<T> {
        options
            .iter()
            .copied()
            .filter(|option| chosen.contains(option))
            .collect()
    }

    fn single<T: Choice>(chosen: &[T]) -> Option<T> {
        let [only] = chosen else {
            return None;
        };
        Some(*only)
    }
}
