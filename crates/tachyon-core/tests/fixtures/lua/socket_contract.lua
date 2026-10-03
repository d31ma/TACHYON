SocketContract = { messages = {}, errors = {} }

function SocketContract:open()
end

function SocketContract:message(_, value)
  self.messages[#self.messages + 1] = value == nil and "nil" or YonJson.encode(value)
end

function SocketContract:error(_, failure)
  self.errors[#self.errors + 1] = tostring(failure)
end

function SocketContract:close()
  print("messages=" .. table.concat(self.messages, ";"))
  print("errors=" .. table.concat(self.errors, ";"))
end

Handler = { YON_FRAMES = true }

function Handler.GET(request)
  YonSocket.run(SocketContract, request)
end
