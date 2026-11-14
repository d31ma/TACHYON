import 'models/Problem.dart';

@ErrorHandler
class OrdersErrorHandler {
  @HandlesError
  Problem signInNeeded(NotSignedIn failure) {
    return const Problem('not_signed_in', '[MS0001]: Sign in again.');
  }

  @HandlesError
  Problem notAllowed(RoleMissing failure) {
    return const Problem('forbidden', '[MS0002]: You may not do this.');
  }

  @HandlesError
  Problem identityDown(IdentityCheckFailed failure) {
    return const Problem('identity_failed', '[MS0003]: Try again later.');
  }

  @HandlesError
  Problem badRequest(RequestRejected failure) {
    return Problem('invalid_request', '[MS0004]: ${failure.part}.${failure.field}');
  }

  @HandlesError
  Problem tooLarge(BodyTooLarge failure) {
    return Problem('too_large', '[MS0005]: ${failure.limitBytes}');
  }

  @HandlesError
  Problem brokenRules(CheckRejected failure) {
    return Problem('invalid_input', '[MS0006]: ${_rules(failure.violations)}');
  }

  @HandlesError
  Problem unexpected(UnexpectedError failure) {
    return const Problem('unexpected', '[MS0000]: Something went wrong.');
  }

  String _rules(List<BrokenRule> violations) {
    return violations.map((violation) => '${violation.field}:${violation.rule}').join(',');
  }
}
