-- @Service
---@class OrderService
OrderService = {
  -- @Needs
  ---@type OrdersClient
  orders_client = nil,
}

---@return SentRequest
function OrderService:replace()
  return self.orders_client:replace("a-7", { item = "hammer" }, "north")
end

---@return SentRequest
function OrderService:change()
  return self.orders_client:change("a-7", { item = "nails" }, "south")
end

---@return SentRequest
function OrderService:remove()
  return self.orders_client:remove("a-7", "duplicate")
end
