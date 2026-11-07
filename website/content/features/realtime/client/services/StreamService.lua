-- @Service
---@class StreamService
StreamService = {
  -- @Needs
  ---@type TicksClient
  ticks_client = nil,
  ---@type Tick[]
  ticks = {},
}

---@param tick Tick
---@return nil
-- @OnEvent
function StreamService:add_tick(tick)
  self.ticks[#self.ticks + 1] = tick
  if #self.ticks > self:_tick_limit() then
    table.remove(self.ticks, 1)
  end
end

---@return integer
function StreamService:_tick_limit()
  return 128
end
