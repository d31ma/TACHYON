-- @RequestRoute("/api/status")
-- @Public
---@class StatusController
StatusController = {}

---@return string
-- @GetRoute
function StatusController:get_status()
  return "ok"
end
