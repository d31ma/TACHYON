-- @Service
---@class OrderService
OrderService = {}

---@return Order[]
function OrderService:list()
  return { { sku = "anvil", level = 4 } }
end
