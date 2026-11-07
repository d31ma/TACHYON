#[Service]
class CheckoutService
  # @return [Integer]
  attr_reader :placed

  def initialize
    @placed = 0
    @events = ApplicationEventPublisher.new
  end

  # @return [void]
  #[OnStart]
  def start
    place
  end

  # @return [void]
  def place
    @placed += 1
    @events.publish_event(PlacedOrder.new(status: "paid", total: 25), true)
  end

  # @return [void]
  #[OnStop]
  def stop
    @placed = 0
  end
end
