-- @RequestRoute("/catalog")
-- @Public
CatalogController = {
-- @Needs
---@type ProductConverter
converter = nil,
}
---@return Summary
-- @GetRoute
function CatalogController:get_catalog()
  local detail = setmetatable({name = "detail", secret = "detail-private"}, Detail)
  return self.converter:map(setmetatable({id = 7, name = "Anvil", detail = detail, items = {detail}, secret = "product-private"}, Product))
end
