#pragma once
#include <string>
#include <vector>
#include "../models/Problem.hpp"

class [[ErrorHandler]] OrdersErrorHandler {
public:
    [[HandlesError]]
    Problem signInNeeded(const NotSignedIn& error) {
        return Problem{"not_signed_in", "[MS0001]: Sign in again."};
    }

    [[HandlesError]]
    Problem notAllowed(const RoleMissing& error) {
        return Problem{"forbidden", "[MS0002]: You may not do this."};
    }

    [[HandlesError]]
    Problem identityDown(const IdentityCheckFailed& error) {
        return Problem{"identity_failed", "[MS0003]: Try again later."};
    }

    [[HandlesError]]
    Problem badRequest(const RequestRejected& error) {
        return Problem{"invalid_request", "[MS0004]: " + error.part + "." + error.field};
    }

    [[HandlesError]]
    Problem tooLarge(const BodyTooLarge& error) {
        return Problem{"too_large", "[MS0005]: " + std::to_string(error.limitBytes)};
    }

    [[HandlesError]]
    Problem brokenRules(const CheckRejected& error) {
        return Problem{"invalid_input", "[MS0006]: " + rules(error.violations)};
    }

    [[HandlesError]]
    Problem unexpected(const UnexpectedError& error) {
        return Problem{"unexpected", "[MS0000]: Something went wrong."};
    }

private:
    static std::string rules(const std::vector<BrokenRule>& violations) {
        std::string joined;
        for (const BrokenRule& violation : violations) {
            joined += "," + violation.field + ":" + violation.rule;
        }
        return joined.erase(0, 1);
    }
};
