#[Service]
class OrderService
  # @return [Array<Order>]
  def list
    [Order.new(sku: "anvil", level: 4)]
  end
end
