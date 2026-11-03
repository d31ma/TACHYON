-- @Service
---@class CatalogService
CatalogService = {
  -- @Needs
  ---@type ProductsClient
  products_client = nil,
  ---@type integer
  count = 0,
}

---@return nil
function CatalogService:load()
  self.count = #self.products_client:list().items
end

---@return nil
function CatalogService:add()
  self.products_client:save({ name = "anvil" })
end
