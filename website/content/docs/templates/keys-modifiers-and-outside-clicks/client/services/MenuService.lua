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

---@return nil
function MenuService:close()
  self.expanded = false
end
