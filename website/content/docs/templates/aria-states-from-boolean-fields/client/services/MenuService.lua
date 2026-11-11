-- @Service
---@class MenuService
MenuService = {
  ---@type boolean
  expanded = false,
}

---@return nil
function MenuService:toggle()
  self.expanded = not self.expanded
end
