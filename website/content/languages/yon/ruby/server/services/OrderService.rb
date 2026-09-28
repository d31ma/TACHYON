#[Service]
class OrderService
  def initialize
    @store = OrderStore.new
  end

  # @return [Array<Order>]
  def list
    @store.all
  end
end
