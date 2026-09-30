-- @RequestRoute("/events")
-- @Public
---@class EventsController
EventsController = {
  -- @Needs
  ---@type TicketService
  ticket_service = nil,
}

---@return nil
-- @StreamRoute
function EventsController:stream_events()
  self:_tickets_for(self.ticket_service:levels())
end

---@param levels integer[]
---@return nil
function EventsController:_tickets_for(levels)
  for _, level in ipairs(levels) do
    coroutine.yield(setmetatable({ sku = "anvil", level = level }, Ticket))
  end
end
