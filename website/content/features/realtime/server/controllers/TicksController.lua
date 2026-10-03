-- @RequestRoute("/ticks")
-- @Public
---@class TicksController
TicksController = {}

---@return nil
-- @StreamRoute
function TicksController:stream_ticks()
  self:_countdown(3)
end

---@param start integer
---@return nil
function TicksController:_countdown(start)
  for level = start, 1, -1 do
    coroutine.yield(setmetatable({ sku = "ANVIL-1", level = level }, Tick))
  end
end
