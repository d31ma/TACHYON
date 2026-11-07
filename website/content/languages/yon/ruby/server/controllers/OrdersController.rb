#[RequestRoute("/orders")]
#[Public]
class OrdersController
  # @return [OrderList]
  #[GetRoute]
  def get_orders
    OrderList.new(orders: OrderService.new.list)
  end
end
