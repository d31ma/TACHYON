use crate::models::Problem;

#[ErrorHandler]
struct OrdersErrorHandler {}

impl OrdersErrorHandler {
    fn new() -> Self {
        Self {}
    }

    #[HandlesError]
    fn sign_in_needed(&self, error: &NotSignedIn) -> Problem {
        Problem { code: String::from("not_signed_in"), message: String::from("[MS0001]: Sign in again.") }
    }

    #[HandlesError]
    fn not_allowed(&self, error: &RoleMissing) -> Problem {
        Problem { code: String::from("forbidden"), message: String::from("[MS0002]: You may not do this.") }
    }

    #[HandlesError]
    fn identity_down(&self, error: &IdentityCheckFailed) -> Problem {
        Problem { code: String::from("identity_failed"), message: String::from("[MS0003]: Try again later.") }
    }

    #[HandlesError]
    fn bad_request(&self, error: &RequestRejected) -> Problem {
        Problem { code: String::from("invalid_request"), message: format!("[MS0004]: {}.{}", error.part, error.field) }
    }

    #[HandlesError]
    fn too_large(&self, error: &BodyTooLarge) -> Problem {
        Problem { code: String::from("too_large"), message: format!("[MS0005]: {}", error.limit_bytes) }
    }

    #[HandlesError]
    fn broken_rules(&self, error: &CheckRejected) -> Problem {
        Problem { code: String::from("invalid_input"), message: format!("[MS0006]: {}", Self::rules(&error.violations)) }
    }

    #[HandlesError]
    fn unexpected(&self, error: &UnexpectedError) -> Problem {
        Problem { code: String::from("unexpected"), message: String::from("[MS0000]: Something went wrong.") }
    }

    fn rules(violations: &[BrokenRule]) -> String {
        let names: Vec<String> = violations.iter().map(|violation| format!("{}:{}", violation.field, violation.rule)).collect();
        names.join(",")
    }
}
