-- @Service
---@class OrderService
OrderService = {
  -- @Needs
  ---@type OrderClient
  order_client = nil,
  ---@type integer
  status = 0,
  ---@type string
  title = "",
  ---@type integer
  mistakes = 0,
}

---@return nil
function OrderService:send()
  local ok, failure = pcall(self.order_client.send, self.order_client, { name = "", level = 12 })
  if ok then
    return
  end
  local problem = BehaviorError.body_of(failure, Problem) or { title = "", errors = {} }
  self.status = BehaviorError.status_of(failure)
  self.title = problem.title or ""
  self.mistakes = #(problem.errors or {})
end
