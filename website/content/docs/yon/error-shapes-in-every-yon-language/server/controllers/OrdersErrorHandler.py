from models import Problem


@error_handler
class OrdersErrorHandler:
    @handles_error
    def sign_in_needed(self, error: NotSignedIn) -> Problem:
        return Problem("not_signed_in", "[MS0001]: Sign in again.")

    @handles_error
    def not_allowed(self, error: RoleMissing) -> Problem:
        return Problem("forbidden", "[MS0002]: You may not do this.")

    @handles_error
    def identity_down(self, error: IdentityCheckFailed) -> Problem:
        return Problem("identity_failed", "[MS0003]: Try again later.")

    @handles_error
    def bad_request(self, error: RequestRejected) -> Problem:
        return Problem("invalid_request", f"[MS0004]: {error.part}.{error.field}")

    @handles_error
    def too_large(self, error: BodyTooLarge) -> Problem:
        return Problem("too_large", f"[MS0005]: {error.limit_bytes}")

    @handles_error
    def broken_rules(self, error: CheckRejected) -> Problem:
        return Problem("invalid_input", f"[MS0006]: {self._rules(error.violations)}")

    @handles_error
    def unexpected(self, error: UnexpectedError) -> Problem:
        return Problem("unexpected", "[MS0000]: Something went wrong.")

    def _rules(self, violations: list[BrokenRule]) -> str:
        return ",".join(f"{violation.field}:{violation.rule}" for violation in violations)
