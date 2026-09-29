-- @RequestRoute("/echo")
-- @Public
---@class EchoController
EchoController = {}

---@param message Message
---@return Message
-- @PostRoute
function EchoController:post_echo(
  -- @FromBody
  message
)
  return message
end
