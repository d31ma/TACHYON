#[RequestRoute("/orders")]
#[Public]
class OrdersController
  # @return [Problem]
  #[GetRoute]
  def get_order
    raise OrderNotFound, "order 42"
  end
end
