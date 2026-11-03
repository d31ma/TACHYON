#[RequestRoute("/orders")]
#[Public]
class OrdersController
  #[Needs]
  # @return [OrderService]
  attr_reader :order_service

  # @param order [Hash]
  # @return [Hash]
  #[PostRoute]
  def post_orders(
    #[FromBody]
    order
  )
    order_service.accept(order)
  end
end
