#[Store]
class OrderStore
  # @return [Array<Order>]
  def all
    [Order.new(sku: "anvil", level: 4)]
  end
end
