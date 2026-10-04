-- @Service
---@class InventoryService
InventoryService = {
  ---@type Product[]
  products = { setmetatable({ name = "Anvil" }, Product), setmetatable({ name = "Rope" }, Product) },
}
