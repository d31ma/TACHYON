using System.Collections.Generic;
using System.Linq;
using Models;

[ErrorHandler]
class OrdersErrorHandler {
    [HandlesError]
    public Problem SignInNeeded(NotSignedIn failure) {
        return new Problem("not_signed_in", "[MS0001]: Sign in again.");
    }

    [HandlesError]
    public Problem NotAllowed(RoleMissing failure) {
        return new Problem("forbidden", "[MS0002]: You may not do this.");
    }

    [HandlesError]
    public Problem IdentityDown(IdentityCheckFailed failure) {
        return new Problem("identity_failed", "[MS0003]: Try again later.");
    }

    [HandlesError]
    public Problem BadRequest(RequestRejected failure) {
        return new Problem("invalid_request", "[MS0004]: " + failure.Part + "." + failure.Field);
    }

    [HandlesError]
    public Problem TooLarge(BodyTooLarge failure) {
        return new Problem("too_large", "[MS0005]: " + failure.LimitBytes);
    }

    [HandlesError]
    public Problem BrokenRules(CheckRejected failure) {
        return new Problem("invalid_input", "[MS0006]: " + Rules(failure.Violations));
    }

    [HandlesError]
    public Problem Unexpected(UnexpectedError failure) {
        return new Problem("unexpected", "[MS0000]: Something went wrong.");
    }

    private static string Rules(IEnumerable<BrokenRule> violations) {
        return string.Join(",", violations.Select(violation => violation.Field + ":" + violation.Rule));
    }
}
