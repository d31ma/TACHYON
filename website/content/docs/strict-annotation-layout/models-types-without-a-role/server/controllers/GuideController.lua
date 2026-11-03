-- @RequestRoute("/guide")
-- @Public
---@class GuideController
GuideController = {}

---@return Guide
-- @GetRoute
function GuideController:get_guide()
  return { title = "Models hold types", pages = 3 }
end
