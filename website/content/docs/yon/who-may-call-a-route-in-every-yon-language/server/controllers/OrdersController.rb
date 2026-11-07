#[RequestRoute("/orders")]
#[SignedIn]
class OrdersController
  # @return [Array<String>]
  #[GetRoute]
  #[Public]
  def get_orders
    ["lamp", "desk"]
  end

  # @return [Array<String>]
  #[PostRoute]
  def post_order
    ["chair"]
  end

  # @return [Array<String>]
  #[DeleteRoute]
  #[Allow(Admin)]
  def delete_orders
    []
  end
end
