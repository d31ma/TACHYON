-- @Helper
---@class MemoryCacheHelper
MemoryCacheHelper = {
  ---@type table<string, table>
  entries = {},
}

---@param key string
---@param execution table
---@return table
function MemoryCacheHelper:get(key, execution)
  return self.entries[key] or { hit = false }
end

---@param key string
---@param value any
---@param execution table
---@return nil
function MemoryCacheHelper:set(key, value, execution)
  self.entries[key] = { hit = true, value = value, expiresAt = execution.expires_at }
end

---@param key string
---@param execution table
---@return nil
function MemoryCacheHelper:delete(key, execution)
  self.entries[key] = nil
end

---@param call table
---@param execution table
---@return boolean
function MemoryCacheHelper.allow(call, execution)
  return true
end
