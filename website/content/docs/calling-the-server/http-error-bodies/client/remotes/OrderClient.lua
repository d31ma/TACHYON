-- @HTTPClient("/api")
---@class OrderClient
OrderClient = {}

---@param order table
---@return OrderInput
-- @PostRequest("/orders")
function OrderClient:send(
  -- @RequestBody
  order
)
end
