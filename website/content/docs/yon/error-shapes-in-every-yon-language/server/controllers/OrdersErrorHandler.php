<?php
use Models\Problem;

#[ErrorHandler]
class OrdersErrorHandler {
    #[HandlesError]
    public function signInNeeded(NotSignedIn $error): Problem {
        return new Problem('not_signed_in', '[MS0001]: Sign in again.');
    }

    #[HandlesError]
    public function notAllowed(RoleMissing $error): Problem {
        return new Problem('forbidden', '[MS0002]: You may not do this.');
    }

    #[HandlesError]
    public function identityDown(IdentityCheckFailed $error): Problem {
        return new Problem('identity_failed', '[MS0003]: Try again later.');
    }

    #[HandlesError]
    public function badRequest(RequestRejected $error): Problem {
        return new Problem('invalid_request', '[MS0004]: ' . $error->part . '.' . $error->field);
    }

    #[HandlesError]
    public function tooLarge(BodyTooLarge $error): Problem {
        return new Problem('too_large', '[MS0005]: ' . $error->limitBytes);
    }

    #[HandlesError]
    public function brokenRules(CheckRejected $error): Problem {
        return new Problem('invalid_input', '[MS0006]: ' . $this->rules($error->violations));
    }

    #[HandlesError]
    public function unexpected(UnexpectedError $error): Problem {
        return new Problem('unexpected', '[MS0000]: Something went wrong.');
    }

    /** @param list<BrokenRule> $violations */
    private function rules(array $violations): string {
        return implode(',', array_map(fn (BrokenRule $violation): string => $violation->field . ':' . $violation->rule, $violations));
    }
}
