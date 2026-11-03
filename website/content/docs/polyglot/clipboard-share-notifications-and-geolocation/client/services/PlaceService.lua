-- @Service
---@class PlaceService
PlaceService = {
  ---@type string
  place = "",
  ---@type string
  problem = "",
}

---@return nil
function PlaceService:locate()
  self:_keep_problem(Geolocation.current_position, Geolocation)
end

---@return nil
function PlaceService:copy()
  self:_keep_problem(Clipboard.write_text, Clipboard, self.place)
end

---@param found PositionFound
---@return nil
-- @OnEvent
function PlaceService:found(found)
  self.place = found.latitude .. ", " .. found.longitude
end

---@param failed BrowserCallFailed
---@return nil
-- @OnEvent
function PlaceService:failed(failed)
  self.problem = failed.message
end

---@param call function
---@param api table
---@param text string|nil
---@return nil
function PlaceService:_keep_problem(call, api, text)
  local ok, refusal = pcall(call, api, text)
  if not ok then
    self.problem = refusal
  end
end
