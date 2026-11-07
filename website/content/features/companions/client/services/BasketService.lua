-- @Service
---@class BasketService
BasketService = {
  ---@type integer
  count = 0,
}

---@return integer
function BasketService:doubled()
  return self.count * 2
end

---@return nil
-- @OnStart
function BasketService:start()
  self.count = 6
end
