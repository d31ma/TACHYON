#[Service]
class OrderService
  #[Needs]
  # @return [OrdersClient]
  attr_reader :orders_client

  # @return [SentRequest]
  def replace
    orders_client.replace("a-7", { "item" => "hammer" }, "north")
  end

  # @return [SentRequest]
  def change
    orders_client.change("a-7", { "item" => "nails" }, "south")
  end

  # @return [SentRequest]
  def remove
    orders_client.remove("a-7", "duplicate")
  end
end
