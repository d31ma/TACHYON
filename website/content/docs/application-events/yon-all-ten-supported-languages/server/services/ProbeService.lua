-- @Service
---@class ProbeService
ProbeService = {
  ---@type ProbeReady|nil
  last = nil,
}

---@param ready ProbeReady
---@return nil
-- @OnEvent
function ProbeService:observe(ready)
  self.last = ready
end

---@return nil
-- @OnStop
function ProbeService:destroy()
  self.last = nil
end
