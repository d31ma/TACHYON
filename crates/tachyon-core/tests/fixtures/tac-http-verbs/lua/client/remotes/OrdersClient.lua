-- @HTTPClient({"path":"/api","adapter":"orders","setup":"OrdersSetup"})
---@class OrdersClient
OrdersClient = {}

---@param id string
---@param order table
---@param tenant string
---@return SentRequest
-- @PutRequest("/orders/{id}")
function OrdersClient:replace(
  -- @PathVariable("id")
  id,
  -- @RequestBody
  order,
  -- @RequestHeader("X-Tenant")
  tenant
)
end

---@param id string
---@param change table
---@param tenant string
---@return SentRequest
-- @PatchRequest("/orders/{id}")
function OrdersClient:change(
  -- @PathVariable("id")
  id,
  -- @RequestBody
  change,
  -- @RequestHeader("X-Tenant")
  tenant
)
end

---@param id string
---@param reason string
---@return SentRequest
-- @DeleteRequest("/orders/{id}")
function OrdersClient:remove(
  -- @PathVariable("id")
  id,
  -- @RequestHeader("X-Reason")
  reason
)
end
