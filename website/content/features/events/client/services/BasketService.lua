-- @Service
---@class BasketService
BasketService = {
  ---@type integer
  count = 0,
  ---@type string
  label = "",
  ---@type string
  _selected = "",
  ---@type Tab[]
  tabs = { setmetatable({ id = "a", name = "A" }, Tab) },
}

---@return nil
function BasketService:increment()
  self.count = self.count + 1
end

---@param id string
---@return nil
function BasketService:select(id)
  self._selected = id
end
