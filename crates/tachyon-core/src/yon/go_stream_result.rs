use crate::go_source::split_top_level;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GoStreamResult {
    Items,
    ItemsOrFailure,
}

impl GoStreamResult {
    const ITEMS: &'static str = "iter.Seq[";
    const ITEMS_OR_FAILURE: &'static str = "iter.Seq2[";
    const FAILURE: &'static str = "error";
    pub(crate) const SHAPE_HELP: &'static str = "A Go @StreamRoute method returns iter.Seq[Model], or iter.Seq2[Model, error] when the stream can fail";

    pub(crate) fn of(results: &str) -> Result<Self, String> {
        let results = results.trim();
        if let Some(inside) = Self::inside(results, Self::ITEMS_OR_FAILURE) {
            return Self::items_or_failure(inside);
        }
        match Self::inside(results, Self::ITEMS) {
            Some(item) if !item.trim().is_empty() => Ok(Self::Items),
            _ => Err(String::from(Self::SHAPE_HELP)),
        }
    }

    pub(crate) const fn wrapper(self) -> &'static str {
        match self {
            Self::Items => "yonStream",
            Self::ItemsOrFailure => "yonStreamWithFailures",
        }
    }

    fn items_or_failure(inside: &str) -> Result<Self, String> {
        let parts = split_top_level(inside, b',');
        match parts.as_slice() {
            [item, failure] if !item.trim().is_empty() && failure.trim() == Self::FAILURE => {
                Ok(Self::ItemsOrFailure)
            }
            _ => Err(String::from(Self::SHAPE_HELP)),
        }
    }

    fn inside<'a>(results: &'a str, opening: &str) -> Option<&'a str> {
        results.strip_prefix(opening)?.strip_suffix(']')
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::GoStreamResult;

    #[test]
    fn a_go_stream_returns_a_sequence_of_items() {
        assert_eq!(
            GoStreamResult::of("iter.Seq[Tick]"),
            Ok(GoStreamResult::Items)
        );
        assert_eq!(
            GoStreamResult::of(" iter.Seq2[map[string]int, error] "),
            Ok(GoStreamResult::ItemsOrFailure)
        );
    }

    #[test]
    fn other_go_stream_results_are_refused_with_the_shape() {
        for results in [
            "[]Tick",
            "<-chan Tick",
            "(iter.Seq[Tick], error)",
            "iter.Seq2[Tick, string]",
            "iter.Seq[]",
        ] {
            let refused = GoStreamResult::of(results).expect_err(results);
            assert!(refused.contains("iter.Seq[Model]"), "{refused}");
        }
    }
}
