-- @TestSetup
---@class OrdersTestSetup
OrdersTestSetup = {}

---@param execution table
---@return table
function OrdersTestSetup.configure(execution)
  local orders = {
    exchange = function(_, request, call)
      local header = request.headers["X-Tenant"] or request.headers["X-Reason"]
      local item = request.body and request.body.item or "none"
      return { method = request.method, path = request.path, header = header, item = item }
    end,
  }
  return { http = { orders = orders } }
end
