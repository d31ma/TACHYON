
class BrokenRule
  attr_reader :field, :rule

  def initialize(value)
    @field = value["field"].to_s
    @rule = value["rule"].to_s
    freeze
  end
end

class YonBuiltInError < StandardError
  attr_reader :reference, :status

  def initialize(status, value)
    super(self.class.name.to_s.split("::").last)
    @status = status
    @reference = value["reference"].to_s
  end
end

class NotSignedIn < YonBuiltInError
  def initialize(value)
    super(401, value)
  end
end

class RoleMissing < YonBuiltInError
  def initialize(value)
    super(403, value)
  end
end

class IdentityCheckFailed < YonBuiltInError
  def initialize(value)
    super(500, value)
  end
end

class RequestRejected < YonBuiltInError
  attr_reader :part, :field, :message

  def initialize(value)
    super(400, value)
    @part = value["part"].to_s
    @field = value["field"].to_s
    @message = value["message"].to_s
  end
end

class BodyTooLarge < YonBuiltInError
  attr_reader :limit_bytes

  def initialize(value)
    super(413, value)
    @limit_bytes = value["limitBytes"].to_i
  end
end

class CheckRejected < YonBuiltInError
  attr_reader :violations, :truncated
  alias_method :truncated?, :truncated

  def initialize(value)
    super(422, value)
    entries = value["violations"].is_a?(Array) ? value["violations"] : []
    @violations = entries.map { |entry| BrokenRule.new(entry) }.freeze
    @truncated = value["truncated"] == true
  end
end

class UnexpectedError < YonBuiltInError
  def initialize(value)
    super(500, value)
  end
end

module YonBuiltInErrors
  def self.build(value)
    type = value.is_a?(Hash) ? types[value["type"]] : nil
    raise TypeError, "Unknown built-in error." if type.nil?

    type.new(value)
  end

  def self.types
    {
      "NotSignedIn" => NotSignedIn,
      "RoleMissing" => RoleMissing,
      "IdentityCheckFailed" => IdentityCheckFailed,
      "RequestRejected" => RequestRejected,
      "BodyTooLarge" => BodyTooLarge,
      "CheckRejected" => CheckRejected,
      "UnexpectedError" => UnexpectedError
    }
  end
end
