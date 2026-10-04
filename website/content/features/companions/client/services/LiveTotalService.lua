-- @Service
---@class LiveTotalService
LiveTotalService = {
  ---@type integer
  seen = 0,
}

---@param changed CartTotal
---@return nil
-- @OnEvent({"replay":true})
function LiveTotalService:changed(changed)
  self.seen = changed.total
end
