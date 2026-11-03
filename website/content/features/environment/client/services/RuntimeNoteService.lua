-- @Service
---@class RuntimeNoteService
RuntimeNoteService = {
  ---@type string
  runtime = "Lua, on the Lua web runtime",
}

---@param development boolean
---@return boolean
function RuntimeNoteService:shows_notes(development)
  return development
end
