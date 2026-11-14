#[RequestRoute("/orders")]
#[Allow(Admin)]
class OrdersController
  #[Needs]
  # @return [OrderService]
  attr_reader :order_service

  # @return [Array<String>]
  #[GetRoute]
  def get_orders
    raise IOError, "the order store is down"
  end

  # @param order [Hash]
  # @return [Hash]
  #[PostRoute]
  def post_order(
    #[FromBody]
    order
  )
    order_service.accept(order)
  end
end
