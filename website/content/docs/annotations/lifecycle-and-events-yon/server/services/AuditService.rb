#[Service]
class AuditService
  # @return [Array<String>]
  attr_reader :log
  # @return [Integer]
  attr_reader :settled

  def initialize
    @log = []
    @settled = 0
  end

  # @param order [PlacedOrder]
  # @return [void]
  #[OnEvent({"id":"audit-log"})]
  #[Order(-10)]
  def record(order)
    @log << order.status
  end

  # @param order [PlacedOrder]
  # @return [SettledOrder]
  #[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
  #[Order(10)]
  def settle(order)
    SettledOrder.new(total: order.total)
  end

  # @param settled [SettledOrder]
  # @return [void]
  #[OnEvent]
  def save_settled(settled)
    @settled = settled.total
  end
end
