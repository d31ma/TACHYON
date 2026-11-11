-- @Service
---@class AttachService
AttachService = {
  ---@type string
  selected = "",
  ---@type string
  name = "",
  ---@type string
  digest = "",
  ---@type string
  status = "",
  ---@type string
  problem = "",
}

---@return nil
function AttachService:choose()
  Files:pick(".pdf,image/*", false)
end

---@return nil
function AttachService:upload()
  Files:upload(self.selected, "PUT", "https://uploads.example.com/inbox", "content-type: application/octet-stream")
end

---@param picked FilePicked
---@return nil
-- @OnEvent
function AttachService:picked(picked)
  self.name = picked.name .. ", " .. tostring(picked.size) .. " bytes"
  self.digest = picked.sha256
  self.selected = picked.id
end

---@param finished UploadFinished
---@return nil
-- @OnEvent
function AttachService:finished(finished)
  self.status = "HTTP " .. tostring(finished.status) .. " " .. finished.error
end
