#[Test(OrdersTestSetup, {"partition":"tac-http-verbs"})]
class OrderServiceTest
  #[Fake]
  attr_accessor :orders_client

  # @return [void]
  def test_put_sends_the_path_the_header_and_the_body
    sent = construct(OrderService).replace
    Assert.equals("PUT", sent.method)
    Assert.equals("/api/orders/a-7", sent.path)
    Assert.equals("north", sent.header)
    Assert.equals("hammer", sent.item)
  end

  # @return [void]
  def test_patch_sends_the_path_the_header_and_the_body
    sent = construct(OrderService).change
    Assert.equals("PATCH", sent.method)
    Assert.equals("/api/orders/a-7", sent.path)
    Assert.equals("south", sent.header)
    Assert.equals("nails", sent.item)
  end

  # @return [void]
  def test_delete_sends_the_path_and_the_header_without_a_body
    sent = construct(OrderService).remove
    Assert.equals("DELETE", sent.method)
    Assert.equals("/api/orders/a-7", sent.path)
    Assert.equals("duplicate", sent.header)
    Assert.equals("none", sent.item)
  end
end
