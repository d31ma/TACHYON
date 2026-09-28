#[allow(dead_code)]
struct Assert;
#[allow(dead_code)]
impl Assert {
    const NOTHING_THROWN: &'static str = "Expected an error to be thrown, but nothing was thrown.";
    const MESSAGE_SEPARATOR: &'static str = " ==> ";
    const PANICKED: &'static str = "The test case panicked.";

    fn equals<E: std::fmt::Debug, A: std::fmt::Debug + PartialEq<E>>(expected: E, actual: A) {
        Self::equals_with_message(expected, actual, "");
    }

    fn equals_with_message<E: std::fmt::Debug, A: std::fmt::Debug + PartialEq<E>>(
        expected: E,
        actual: A,
        message: &str,
    ) {
        let failure = format!("expected: <{expected:?}> but was: <{actual:?}>");
        Self::check(actual == expected, &failure, message);
    }

    fn is_true(condition: bool) {
        Self::is_true_with_message(condition, "");
    }

    fn is_true_with_message(condition: bool, message: &str) {
        Self::check(condition, "expected: <true> but was: <false>", message);
    }

    fn throws<R>(action: impl FnOnce() -> R)
    where
        CaseOutcome: From<R>,
    {
        Self::throws_with_message(action, "");
    }

    fn throws_with_message<R>(action: impl FnOnce() -> R, message: &str)
    where
        CaseOutcome: From<R>,
    {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            CaseOutcome::from(action()).failure()
        }));
        let nothing_thrown = outcome.is_ok_and(|failure| failure.is_none());
        Self::check(!nothing_thrown, Self::NOTHING_THROWN, message);
    }

    fn check(passed: bool, failure: &str, message: &str) {
        if !passed {
            Self::fail(failure, message);
        }
    }

    fn fail(failure: &str, message: &str) {
        panic!("{}", Self::failure_text(failure, message));
    }

    fn failure_text(failure: &str, message: &str) -> String {
        if message.is_empty() {
            return failure.to_owned();
        }
        format!("{message}{}{failure}", Self::MESSAGE_SEPARATOR)
    }

    fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
        let text = payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned());
        text.or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| String::from(Self::PANICKED))
    }
}
