-- @Service
---@class CardService
CardService = {
  ---@type string
  name = "Anvil",
  ---@type string
  status = "waiting",
}

---@return nil
-- @OnStart
function CardService:activate()
  self.status = "active"
end
