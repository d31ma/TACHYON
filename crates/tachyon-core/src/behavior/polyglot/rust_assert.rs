#[allow(dead_code)]
struct Assert;
#[allow(dead_code)]
impl Assert {
    const NOTHING_THROWN: &'static str = "Expected an error to be thrown, but nothing was thrown.";
    const MESSAGE_SEPARATOR: &'static str = " ==> ";
    fn equals<E: std::fmt::Debug, A: std::fmt::Debug + PartialEq<E>>(expected: E, actual: A) {
        Self::equals_with_message(expected, actual, "");
    }
    fn equals_with_message<E: std::fmt::Debug, A: std::fmt::Debug + PartialEq<E>>(
        expected: E,
        actual: A,
        message: &str,
    ) {
        if actual != expected {
            Self::fail(
                &format!("expected: <{expected:?}> but was: <{actual:?}>"),
                message,
            );
        }
    }
    fn is_true(condition: bool) {
        Self::is_true_with_message(condition, "");
    }
    fn is_true_with_message(condition: bool, message: &str) {
        if !condition {
            Self::fail("expected: <true> but was: <false>", message);
        }
    }
    fn throws<R: CaseOutcome>(action: impl FnOnce() -> R) {
        Self::throws_with_message(action, "");
    }
    fn throws_with_message<R: CaseOutcome>(action: impl FnOnce() -> R, message: &str) {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| action().failure()));
        if matches!(outcome, Ok(None)) {
            Self::fail(Self::NOTHING_THROWN, message);
        }
    }
    fn fail(failure: &str, message: &str) {
        if message.is_empty() {
            panic!("{failure}");
        }
        panic!("{message}{}{failure}", Self::MESSAGE_SEPARATOR);
    }
    fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
        if let Some(text) = payload.downcast_ref::<&str>() {
            return (*text).to_owned();
        }
        payload
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_else(|| String::from("The test case panicked."))
    }
}
#[allow(dead_code)]
trait CaseOutcome {
    fn failure(self) -> Option<String>;
}
impl CaseOutcome for () {
    fn failure(self) -> Option<String> {
        None
    }
}
impl<T> CaseOutcome for Result<T, BehaviorError> {
    fn failure(self) -> Option<String> {
        self.err().map(|error| error.message.to_owned())
    }
}
impl<T> CaseOutcome for Result<T, String> {
    fn failure(self) -> Option<String> {
        self.err()
    }
}
impl<T> CaseOutcome for Result<T, &'static str> {
    fn failure(self) -> Option<String> {
        self.err().map(str::to_owned)
    }
}
