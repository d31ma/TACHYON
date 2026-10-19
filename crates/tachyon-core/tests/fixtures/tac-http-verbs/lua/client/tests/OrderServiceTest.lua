-- @Test(OrdersTestSetup, {"partition":"tac-http-verbs"})
---@class OrderServiceTest
OrderServiceTest = {
  -- @Fake
  ---@type OrdersClient
  orders_client = nil,
}

function OrderServiceTest:test_put_sends_the_path_the_header_and_the_body()
  local sent = self:construct(OrderService):replace()
  Assert.equals("PUT", sent.method)
  Assert.equals("/api/orders/a-7", sent.path)
  Assert.equals("north", sent.header)
  Assert.equals("hammer", sent.item)
end

function OrderServiceTest:test_patch_sends_the_path_the_header_and_the_body()
  local sent = self:construct(OrderService):change()
  Assert.equals("PATCH", sent.method)
  Assert.equals("/api/orders/a-7", sent.path)
  Assert.equals("south", sent.header)
  Assert.equals("nails", sent.item)
end

function OrderServiceTest:test_delete_sends_the_path_and_the_header_without_a_body()
  local sent = self:construct(OrderService):remove()
  Assert.equals("DELETE", sent.method)
  Assert.equals("/api/orders/a-7", sent.path)
  Assert.equals("duplicate", sent.header)
  Assert.equals("none", sent.item)
end
