-- @Setup
---@class OverrideSetup
OverrideSetup = {}

---@param execution table
---@return table
function OverrideSetup.configure(execution)
  return { http = { override = OverrideSetup } }
end

---@param request table
---@param execution table
---@return table
function OverrideSetup:exchange(request, execution)
  return { message = "Custom adapter wins" }
end
