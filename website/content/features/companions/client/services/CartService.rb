#[Service]
class CartService
  # @return [Integer]
  attr_reader :total

  def initialize
    @total = 0
    @events = ApplicationEventPublisher.new
  end

  # @return [void]
  def add
    @total += 1
    @events.publish_event(CartTotal.new(total: @total), true)
  end
end
