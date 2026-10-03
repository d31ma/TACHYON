TacRuntimeContract = {}
TacRuntimeContract.__index = TacRuntimeContract

function TacRuntimeContract.new()
  return setmetatable({ failures = {} }, TacRuntimeContract)
end

function TacRuntimeContract:expect(passed, message)
  if not passed then
    self.failures[#self.failures + 1] = message
  end
end

function TacRuntimeContract:check_arguments()
  local owner = { instance = {
    pair = function(_, first, second)
      return { first == nil, second }
    end,
  } }
  local answer = TacServices.call(owner, "pair", YonJson.decode("[null,5]"))
  self:expect(YonJson.encode(answer.value) == "[true,5]", "arguments [null,5] reach the method as " .. YonJson.encode(answer.value))
end

function TacRuntimeContract:check_values()
  self:expect(TacValues.copy(YonNull) == YonNull, "a copied null stops being null")
  self:expect(YonJson.encode(TacValues.copy(YonJson.decode("{\"meta\":{}}"))) == "{\"meta\":{}}", "a copied empty object becomes an array")
end

function TacRuntimeContract:run()
  self:check_arguments()
  self:check_values()
  if #self.failures > 0 then
    error(table.concat(self.failures, "\n"), 0)
  end
  print("Tac Lua runtime contracts passed")
end

TacRuntimeContract.new():run()
