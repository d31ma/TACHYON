-- @RequestRoute("/ticks")
-- @Public
---@class TicksController
TicksController = {}

---@return nil
-- @StreamRoute
function TicksController:stream_ticks()
  self:_ticks_up_to(3)
end

---@param last integer
---@return nil
function TicksController:_ticks_up_to(last)
  for level = 1, last do
    coroutine.yield(setmetatable({ sku = "ANVIL-1", level = level }, Tick))
  end
end
