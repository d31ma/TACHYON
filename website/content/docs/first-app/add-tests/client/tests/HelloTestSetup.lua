-- @TestSetup
---@class HelloTestSetup
HelloTestSetup = {}

---@param execution table
---@return table
function HelloTestSetup.configure(execution)
  return { http = { greeting = HelloTestSetup } }
end

---@param request table
---@param execution table
---@return table
function HelloTestSetup:exchange(request, execution)
  return { message = "Hi" }
end
