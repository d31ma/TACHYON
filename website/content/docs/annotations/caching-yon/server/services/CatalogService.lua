-- @Service
-- @CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
---@class CatalogService
CatalogService = {
  ---@type integer
  reads = 0,
}

---@return nil
-- @OnStart
function CatalogService:open()
  self.reads = 0
  self.behaviors = Behaviors.create(CatalogSetup, { partition = "public-catalog", signal = BehaviorSignal.new() })
end

---@param sku string
---@return table
-- @Cached
function CatalogService:find(sku)
  self.reads = self.reads + 1
  return self:_entry(sku, self.reads)
end

---@param sku string
---@param reads integer
---@return table
-- @UpdateCache({"method":"find"})
function CatalogService:write(sku, reads)
  return self:_entry(sku, reads)
end

---@param sku string
---@return table
-- @ClearCache({"method":"find"})
function CatalogService:invalidate(sku)
  return self:_entry(sku, self.reads)
end

---@param sku string
---@param reads integer
---@return table
function CatalogService:_entry(sku, reads)
  return { sku = sku, reads = reads }
end
