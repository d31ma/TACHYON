import (
	"fmt"
	"strings"
)

// @ErrorHandler
type OrdersErrorHandler struct{}

// @HandlesError
func (handler *OrdersErrorHandler) SignInNeeded(failure *NotSignedIn) Problem {
	return Problem{Code: "not_signed_in", Message: "[MS0001]: Sign in again."}
}

// @HandlesError
func (handler *OrdersErrorHandler) NotAllowed(failure *RoleMissing) Problem {
	return Problem{Code: "forbidden", Message: "[MS0002]: You may not do this."}
}

// @HandlesError
func (handler *OrdersErrorHandler) IdentityDown(failure *IdentityCheckFailed) Problem {
	return Problem{Code: "identity_failed", Message: "[MS0003]: Try again later."}
}

// @HandlesError
func (handler *OrdersErrorHandler) BadRequest(failure *RequestRejected) Problem {
	return Problem{Code: "invalid_request", Message: "[MS0004]: " + failure.Part + "." + failure.Field}
}

// @HandlesError
func (handler *OrdersErrorHandler) TooLarge(failure *BodyTooLarge) Problem {
	return Problem{Code: "too_large", Message: fmt.Sprintf("[MS0005]: %d", failure.LimitBytes)}
}

// @HandlesError
func (handler *OrdersErrorHandler) BrokenRules(failure *CheckRejected) Problem {
	return Problem{Code: "invalid_input", Message: "[MS0006]: " + handler.rules(failure.Violations)}
}

// @HandlesError
func (handler *OrdersErrorHandler) Unexpected(failure *UnexpectedError) Problem {
	return Problem{Code: "unexpected", Message: "[MS0000]: Something went wrong."}
}

func (handler *OrdersErrorHandler) rules(violations []BrokenRule) string {
	names := make([]string, 0, len(violations))
	for _, violation := range violations {
		names = append(names, violation.Field+":"+violation.Rule)
	}
	return strings.Join(names, ",")
}
