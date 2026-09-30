---@class CatalogService
CatalogService = {
  ---@type integer
  count = 0,
  ---@type ApplicationEventPublisher
  application_event_publisher = nil,
}

---@return nil
-- @OnStart
function CatalogService:initialize_count()
  self:_announce()
end

---@param value CatalogCount
---@return nil
-- @OnEvent
function CatalogService:changed(value)
  self.count = value.count
end

---@return nil
-- @OnStop
function CatalogService:cleanup()
  self.count = 0
end

---@return nil
function CatalogService:_announce()
  self.application_event_publisher:publish_event(setmetatable({ count = 5 }, CatalogCount))
end
