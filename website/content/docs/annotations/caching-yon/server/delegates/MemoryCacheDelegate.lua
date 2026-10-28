-- @Delegate
---@class MemoryCacheDelegate
MemoryCacheDelegate = {
  ---@type table<string, table>
  entries = {},
}

---@param key string
---@param execution table
---@return table
function MemoryCacheDelegate:get(key, execution)
  return self.entries[key] or { hit = false }
end

---@param key string
---@param value any
---@param execution table
---@return nil
function MemoryCacheDelegate:set(key, value, execution)
  self.entries[key] = { hit = true, value = value, expiresAt = execution.expires_at }
end

---@param key string
---@param execution table
---@return nil
function MemoryCacheDelegate:delete(key, execution)
  self.entries[key] = nil
end

---@param call table
---@param execution table
---@return boolean
function MemoryCacheDelegate.allow(call, execution)
  return true
end
