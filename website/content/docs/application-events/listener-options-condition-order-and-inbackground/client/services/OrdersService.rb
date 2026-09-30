#[Service]
class OrdersService
  # @return [Array<String>]
  attr_reader :log
  # @return [Integer]
  attr_reader :settled
  # @return [Integer]
  attr_reader :audited

  def initialize
    @log = []
    @settled = 0
    @audited = 0
    @events = ApplicationEventPublisher.new
  end

  # @param total [Integer]
  # @return [void]
  def place(total)
    @events.publish_event(PlacedOrder.new(status: "paid", total: total))
  end

  # @param order [PlacedOrder]
  # @return [void]
  #[OnEvent({"id":"orders-log"})]
  #[Order(-10)]
  def record(order)
    @log = [*@log, order.status]
  end

  # @param order [PlacedOrder]
  # @return [SettledOrder]
  #[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
  def settle_large_order(order)
    @settled += 1
    SettledOrder.new(total: order.total)
  end

  # @param order [SettledOrder]
  # @return [void]
  #[OnEvent]
  #[InBackground]
  def audit(order)
    @audited = order.total
  end
end
