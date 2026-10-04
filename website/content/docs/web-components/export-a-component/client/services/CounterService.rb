#[Service]
class CounterService
  # @return [Integer]
  attr_reader :count

  def initialize
    @count = 0
    @events = ApplicationEventPublisher.new
  end

  # @return [void]
  def add
    @count += 1
    @events.publish_event(CountChanged.new(count: @count))
  end
end
