#[ErrorHandler]
class OrdersErrorHandler
  # @param error [NotSignedIn]
  # @return [Problem]
  #[HandlesError]
  def sign_in_needed(error)
    Problem.new(code: "not_signed_in", message: "[MS0001]: Sign in again.")
  end

  # @param error [RoleMissing]
  # @return [Problem]
  #[HandlesError]
  def not_allowed(error)
    Problem.new(code: "forbidden", message: "[MS0002]: You may not do this.")
  end

  # @param error [IdentityCheckFailed]
  # @return [Problem]
  #[HandlesError]
  def identity_down(error)
    Problem.new(code: "identity_failed", message: "[MS0003]: Try again later.")
  end

  # @param error [RequestRejected]
  # @return [Problem]
  #[HandlesError]
  def bad_request(error)
    Problem.new(code: "invalid_request", message: "[MS0004]: #{error.part}.#{error.field}")
  end

  # @param error [BodyTooLarge]
  # @return [Problem]
  #[HandlesError]
  def too_large(error)
    Problem.new(code: "too_large", message: "[MS0005]: #{error.limit_bytes}")
  end

  # @param error [CheckRejected]
  # @return [Problem]
  #[HandlesError]
  def broken_rules(error)
    Problem.new(code: "invalid_input", message: "[MS0006]: #{rules(error.violations)}")
  end

  # @param error [UnexpectedError]
  # @return [Problem]
  #[HandlesError]
  def unexpected(error)
    Problem.new(code: "unexpected", message: "[MS0000]: Something went wrong.")
  end

  private

  # @param violations [Array<BrokenRule>]
  # @return [String]
  def rules(violations)
    violations.map { |violation| "#{violation.field}:#{violation.rule}" }.join(",")
  end
end
