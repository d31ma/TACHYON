import java.util.List;
import java.util.stream.Collectors;
import models.Problem;

@ErrorHandler
class OrdersErrorHandler {
    @HandlesError
    Problem signInNeeded(NotSignedIn error) {
        return new Problem("not_signed_in", "[MS0001]: Sign in again.");
    }

    @HandlesError
    Problem notAllowed(RoleMissing error) {
        return new Problem("forbidden", "[MS0002]: You may not do this.");
    }

    @HandlesError
    Problem identityDown(IdentityCheckFailed error) {
        return new Problem("identity_failed", "[MS0003]: Try again later.");
    }

    @HandlesError
    Problem badRequest(RequestRejected error) {
        return new Problem("invalid_request", "[MS0004]: " + error.getPart() + "." + error.getField());
    }

    @HandlesError
    Problem tooLarge(BodyTooLarge error) {
        return new Problem("too_large", "[MS0005]: " + error.getLimitBytes());
    }

    @HandlesError
    Problem brokenRules(CheckRejected error) {
        return new Problem("invalid_input", "[MS0006]: " + rules(error.getViolations()));
    }

    @HandlesError
    Problem unexpected(UnexpectedError error) {
        return new Problem("unexpected", "[MS0000]: Something went wrong.");
    }

    private String rules(List<BrokenRule> violations) {
        return violations.stream()
            .map(violation -> violation.getField() + ":" + violation.getRule())
            .collect(Collectors.joining(","));
    }
}
