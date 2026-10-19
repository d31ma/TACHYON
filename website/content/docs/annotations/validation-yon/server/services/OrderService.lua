-- @Service
---@class OrderService
OrderService = {}

---@param order table
---@return table
function OrderService:accept(
  -- @Check(OrderInput)
  order
)
  return order
end
