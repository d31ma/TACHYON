#[allow(dead_code)]
struct CaseOutcome(Option<String>);
#[allow(dead_code)]
impl CaseOutcome {
    fn failure(self) -> Option<String> {
        self.0
    }
}
impl From<()> for CaseOutcome {
    fn from((): ()) -> Self {
        Self(None)
    }
}
impl<T> From<Result<T, BehaviorError>> for CaseOutcome {
    fn from(result: Result<T, BehaviorError>) -> Self {
        Self(result.err().map(|error| error.message.to_owned()))
    }
}
impl<T> From<Result<T, String>> for CaseOutcome {
    fn from(result: Result<T, String>) -> Self {
        Self(result.err())
    }
}
impl<T> From<Result<T, &'static str>> for CaseOutcome {
    fn from(result: Result<T, &'static str>) -> Self {
        Self(result.err().map(str::to_owned))
    }
}
