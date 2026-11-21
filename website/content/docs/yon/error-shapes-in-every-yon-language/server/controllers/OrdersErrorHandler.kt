import models.Problem

@ErrorHandler
class OrdersErrorHandler {
    @HandlesError
    fun signInNeeded(error: NotSignedIn): Problem {
        return Problem("not_signed_in", "[MS0001]: Sign in again.")
    }

    @HandlesError
    fun notAllowed(error: RoleMissing): Problem {
        return Problem("forbidden", "[MS0002]: You may not do this.")
    }

    @HandlesError
    fun identityDown(error: IdentityCheckFailed): Problem {
        return Problem("identity_failed", "[MS0003]: Try again later.")
    }

    @HandlesError
    fun badRequest(error: RequestRejected): Problem {
        return Problem("invalid_request", "[MS0004]: ${error.part}.${error.field}")
    }

    @HandlesError
    fun tooLarge(error: BodyTooLarge): Problem {
        return Problem("too_large", "[MS0005]: ${error.limitBytes}")
    }

    @HandlesError
    fun brokenRules(error: CheckRejected): Problem {
        return Problem("invalid_input", "[MS0006]: ${rules(error.violations)}")
    }

    @HandlesError
    fun unexpected(error: UnexpectedError): Problem {
        return Problem("unexpected", "[MS0000]: Something went wrong.")
    }

    private fun rules(violations: List<BrokenRule>): String {
        return violations.joinToString(",") { violation -> "${violation.field}:${violation.rule}" }
    }
}
