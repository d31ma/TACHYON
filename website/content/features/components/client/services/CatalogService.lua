-- @Service
---@class CatalogService
CatalogService = {
  ---@type Product[]
  products = {},
}

---@return nil
-- @OnStart
function CatalogService:start()
  self.products = {
    setmetatable({ id = "anvil", name = "Anvil", price = "£42.00" }, Product),
    setmetatable({ id = "rope", name = "Rope", price = "£8.00" }, Product),
  }
end
