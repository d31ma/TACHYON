-- @Service
---@class InventoryService
InventoryService = {
  ---@type InventoryRecord[]
  records = {},
}

---@param record InventoryRecord
---@return nil
-- @OnEvent
function InventoryService:add_record(record)
  self.records[#self.records + 1] = record
  if #self.records > 64 then
    table.remove(self.records, 1)
  end
end
