-- @Service
---@class CatalogService
CatalogService = {
  ---@type string
  title = "Catalog",
  ---@type integer
  count = 6,
  ---@type Product
  product = setmetatable({ name = "Anvil", price = "£42.00" }, Product),
}
