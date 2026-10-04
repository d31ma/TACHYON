use crate::go_source::split_top_level;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GoStreamResult {
    wrapper: &'static str,
}

impl GoStreamResult {
    const ITEMS: &'static str = "iter.Seq[";
    const ITEMS_OR_FAILURE: &'static str = "iter.Seq2[";
    const FAILURE: &'static str = "error";
    const ITEM_STREAM: Self = Self {
        wrapper: "yonStream",
    };
    const FAILING_STREAM: Self = Self {
        wrapper: "yonStreamWithFailures",
    };
    pub(crate) const SHAPE_HELP: &'static str = "A Go @StreamRoute method returns iter.Seq[Model], or iter.Seq2[Model, error] when the stream can fail";

    pub(crate) fn of(results: &str) -> Result<Self, String> {
        let results = results.trim();
        Self::items_or_failure(results)
            .or_else(|| Self::items(results))
            .ok_or_else(|| String::from(Self::SHAPE_HELP))
    }

    pub(crate) const fn wrapper(self) -> &'static str {
        self.wrapper
    }

    fn items(results: &str) -> Option<Self> {
        let item = Self::inside(results, Self::ITEMS)?;
        Self::names_a_type(item).then_some(Self::ITEM_STREAM)
    }

    fn items_or_failure(results: &str) -> Option<Self> {
        let inside = Self::inside(results, Self::ITEMS_OR_FAILURE)?;
        let [item, failure] = split_top_level(inside, b',')[..] else {
            return None;
        };
        let fits = Self::names_a_type(item) & (failure.trim() == Self::FAILURE);
        fits.then_some(Self::FAILING_STREAM)
    }

    fn names_a_type(text: &str) -> bool {
        !text.trim().is_empty()
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
            GoStreamResult::of("iter.Seq[Tick]").map(GoStreamResult::wrapper),
            Ok("yonStream")
        );
        assert_eq!(
            GoStreamResult::of(" iter.Seq2[map[string]int, error] ").map(GoStreamResult::wrapper),
            Ok("yonStreamWithFailures")
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
