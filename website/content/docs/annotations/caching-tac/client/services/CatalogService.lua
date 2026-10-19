-- @Service
-- @CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
---@class CatalogService
CatalogService = {
  ---@type string
  sku = "anvil",
  ---@type integer
  reads = 0,
}

---@return nil
-- @OnStart
function CatalogService:open()
  self.behaviors = Behaviors.create(CatalogSetup, { partition = "public-catalog", signal = BehaviorSignal.new() })
end

---@param sku string
---@return table
-- @Cached
function CatalogService:find(sku)
  self.reads = self.reads + 1
  return self:entry(sku, self.reads)
end

---@param sku string
---@param reads integer
---@return table
-- @UpdateCache({"method":"find"})
function CatalogService:write(sku, reads)
  return self:entry(sku, reads)
end

---@param sku string
---@return table
-- @ClearCache({"method":"find"})
function CatalogService:invalidate(sku)
  return self:entry(sku, self.reads)
end

---@param sku string
---@param reads integer
---@return table
function CatalogService:entry(sku, reads)
  return { sku = sku, reads = reads }
end
