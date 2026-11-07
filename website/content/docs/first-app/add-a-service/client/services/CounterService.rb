#[Service]
class CounterService
  # @return [Integer]
  attr_reader :count

  def initialize
    @count = 0
  end

  # @return [void]
  def increment
    @count += 1
  end
end
