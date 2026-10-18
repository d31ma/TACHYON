-- @Service
---@class ChatService
ChatService = {
  -- @Needs
  ---@type ChatClient
  chat_client = nil,
  ---@type ChatMessage[]
  messages = {},
}

---@param message ChatMessage
---@return nil
-- @OnEvent
function ChatService:add_message(message)
  self.messages[#self.messages + 1] = message
  if #self.messages > self:_message_limit() then
    table.remove(self.messages, 1)
  end
end

---@return nil
function ChatService:say_hello()
  self.chat_client:send(setmetatable({ text = "Hello from a tab" }, ChatMessage))
end

---@return integer
function ChatService:_message_limit()
  return 50
end
