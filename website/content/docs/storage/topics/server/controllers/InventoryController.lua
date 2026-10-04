-- @RequestRoute("/inventory")
-- @Public
---@class InventoryController
InventoryController = {
  ---@type ApplicationEventPublisher
  application_event_publisher = nil,
}

---@param record InventoryRecord
---@return InventoryRecord
-- @PostRoute
function InventoryController:post_record(
  -- @FromBody
  record
)
  self.application_event_publisher:publish_event(record)
  return record
end
