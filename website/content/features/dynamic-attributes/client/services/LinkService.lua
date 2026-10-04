-- @Service
---@class LinkService
LinkService = {
  ---@type Topic
  topic = setmetatable({ slug = "routing", title = "Routing" }, Topic),
  ---@type string
  state = "ready",
  ---@type string
  message = "Everything is up to date.",
}
