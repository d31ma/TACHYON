---@class BasketService
BasketService = {
  ---@type integer
  count = 0,
  ---@type string
  label = "",
}

---@return nil
-- @OnStart
function BasketService:start()
  self.count = 6
  self.label = "Lua"
end

---@return integer
function BasketService:doubled()
  return self.count * 2
end
