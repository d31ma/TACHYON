class AuditService
  # @return [Integer]
  attr_reader :settled

  def initialize
    @records = 0
    @settled = 0
    @audited = 0
  end

  # @param order [PlacedOrder]
  # @return [void]
  #[OnEvent({"id":"audit-log","replay":true})]
  #[Order(-10)]
  def record(order)
    @records += 1
  end

  # @param order [PlacedOrder]
  # @return [SettledOrder]
  #[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]
  #[Order(10)]
  def settle(order)
    SettledOrder.new(total: order.total)
  end

  # @param order [SettledOrder]
  # @return [void]
  #[OnEvent]
  def save_settled(order)
    @settled = order.total
  end

  # @param order [SettledOrder]
  # @return [void]
  #[OnEvent]
  #[InBackground]
  def audit(order)
    @audited = order.total
  end
end
