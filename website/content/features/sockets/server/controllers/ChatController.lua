-- @SocketRoute("/chat")
-- @Public
---@class ChatController
ChatController = {}

---@param session SocketSession
---@return nil
-- @OnOpen
function ChatController:on_open(session)
  session:send(setmetatable({ text = "Welcome to the room" }, ChatMessage))
end

---@param session SocketSession
---@param message ChatMessage
---@return nil
-- @OnMessage
function ChatController:on_message(session, message)
  self:_send_to_everyone(session:open_sessions(), message)
end

---@param peers SocketPeer[]
---@param message ChatMessage
---@return nil
function ChatController:_send_to_everyone(peers, message)
  for _, peer in ipairs(peers) do
    peer:send(message)
  end
end
