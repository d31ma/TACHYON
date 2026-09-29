-- @ErrorHandler
---@class OrdersErrorHandler
OrdersErrorHandler = {}

---@param failure OrderNotFound
---@return Problem
-- @HandlesError
-- @ResponseStatus(404)
function OrdersErrorHandler:missing(failure)
  return { code = "order-not-found" }
end
