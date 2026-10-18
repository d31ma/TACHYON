-- @Service
---@class OrderService
OrderService = {
  ---@type string
  status = "ready",
}

---@param order OrderInput
---@return OrderInput
function OrderService:accept(
  -- @Check(OrderInput)
  order
)
  return order
end
