-- @Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})
---@class EntryStore
EntryStore = {}

---@param status string
---@return table[]
function EntryStore:find_by_status(status)
end

---@param status string
---@return integer
function EntryStore:count_by_status(status)
end
