-- @Service
---@class RenameService
RenameService = {
  ---@type boolean
  editing = false,
  ---@type string
  problem = "",
}

---@return nil
function RenameService:open()
  self.editing = true
  self:_keep_problem(Focus.trap, "dialog")
  self:_keep_problem(Focus.move, "name")
end

---@return nil
function RenameService:close()
  self.editing = false
  self:_keep_problem(Focus.release)
end

---@param failed BrowserCallFailed
---@return nil
-- @OnEvent
function RenameService:failed(failed)
  self.problem = failed.message
end

---@param call function
---@param name string|nil
---@return nil
function RenameService:_keep_problem(call, name)
  local ok, refusal = pcall(call, Focus, name)
  if not ok then
    self.problem = refusal
  end
end
