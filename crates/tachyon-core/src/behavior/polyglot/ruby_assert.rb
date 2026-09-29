
unless defined?(Assert)
  class Assert
    NOTHING_THROWN = "Expected an error to be thrown, but nothing was thrown."
    MESSAGE_SEPARATOR = " ==> "

    class Failure < StandardError
    end

    def self.equals(expected, actual, message = nil)
      check(expected == actual, "expected: <#{show(expected)}> but was: <#{show(actual)}>", message)
    end

    def self.is_true(condition, message = nil)
      check(condition == true, "expected: <true> but was: <#{show(condition)}>", message)
    end

    def self.throws(message = nil)
      yield
    rescue StandardError => error
      error
    else
      raise Failure, text(NOTHING_THROWN, message)
    end

    def self.check(passed, failure, message)
      raise Failure, text(failure, message) unless passed
    end

    def self.text(failure, message)
      return failure if message.nil?

      "#{message}#{MESSAGE_SEPARATOR}#{failure}"
    end

    def self.show(value)
      return value if value.is_a?(String)

      value.inspect
    end

    private_class_method :check, :text, :show
  end
end
