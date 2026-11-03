-- @Service
---@class TicketService
TicketService = {
  ---@type string
  ticket = "",
  ---@type number
  issued_at = 0.0,
}

---@return nil
function TicketService:issue()
  self.ticket = SecureRandom:uuid()
  self.issued_at = Clock:now_ms()
end
