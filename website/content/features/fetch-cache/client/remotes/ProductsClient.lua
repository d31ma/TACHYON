-- @CacheSettings({"name":"products","ttlMs":60000,"maxBytes":65536,"maxEntries":16})
-- @HTTPClient
---@class ProductsClient
ProductsClient = {}

---@return table
-- @Cached
-- @GetRequest("/api/products")
function ProductsClient:list()
end

---@param product table
---@return table
-- @ClearCache({"method":"list"})
-- @PostRequest("/api/products")
function ProductsClient:save(
  -- @RequestBody
  product
)
end
