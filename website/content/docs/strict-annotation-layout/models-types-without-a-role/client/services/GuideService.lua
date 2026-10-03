-- @Service
---@class GuideService
GuideService = {
  ---@type string
  title = "",
  ---@type integer
  pages = 0,
}

---@return nil
-- @OnStart
function GuideService:start()
  local guide = setmetatable({ title = "Models hold types", pages = 3 }, Guide)
  self.title = guide.title
  self.pages = guide.pages
end
