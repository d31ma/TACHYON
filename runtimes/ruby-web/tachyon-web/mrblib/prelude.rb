class LoadError < ScriptError
end

module Kernel
  private

  def require(name)
    return false if %w[json set time].include?(name.to_s)
    raise LoadError, "cannot load such file -- #{name} (Tac Ruby services are one file)"
  end

  def require_relative(name)
    raise LoadError, "cannot load such file -- #{name} (Tac Ruby services are one file)"
  end

  def warn(*messages)
    messages.each { |message| TachyonWeb.host_write("#{message}\n") }
    nil
  end

  def sleep(*)
    raise NotImplementedError, "sleep would freeze the page; Tac Ruby calls are synchronous"
  end
end

class TachyonConsole
  def write(*values)
    values.each { |value| TachyonWeb.host_write(value.to_s) }
    values.sum { |value| value.to_s.bytesize }
  end

  def puts(*values) = Kernel.instance_method(:puts).bind_call(self, *values)
  def print(*values) = Kernel.instance_method(:print).bind_call(self, *values)
  def flush = self
  def sync = true
  def sync=(value)
    value
  end
end

STDOUT = TachyonConsole.new
STDERR = TachyonConsole.new
$stdout = STDOUT
$stderr = STDERR

class Range
  def step(amount = 1, &block)
    raise ArgumentError, "step can't be 0" if self.begin.is_a?(Numeric) && amount == 0
    return to_enum(:step, amount) unless block
    if self.begin.is_a?(Numeric)
      value = self.begin
      while step_includes?(value, amount)
        block.call(value)
        value += amount
      end
    else
      each_with_index { |value, index| block.call(value) if index % amount == 0 }
    end
    self
  end
  alias % step

  private

  def step_includes?(value, amount)
    return true if self.end.nil?
    if amount < 0
      return exclude_end? ? value > self.end : value >= self.end
    end
    exclude_end? ? value < self.end : value <= self.end
  end
end

class Exception
  def detailed_message(highlight: false, **) = "#{message} (#{self.class})"
  def full_message(highlight: false, order: :top) = "#{backtrace&.first || '-'}: #{detailed_message}\n"
end
