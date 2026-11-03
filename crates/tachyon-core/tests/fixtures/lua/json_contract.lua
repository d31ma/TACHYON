JsonContract = {
  ROUND_TRIPS = {
    "[1,null,2]",
    "[null]",
    "[[null],{\"a\":[null,true]}]",
    "{}",
    "{\"meta\":{}}",
    "[{},[]]",
  },
  ACCEPTED = {
    ["-0"] = 0,
    ["1.5e3"] = 1500.0,
    ["2E+2"] = 200.0,
    ["0.25"] = 0.25,
    ["\"\\ud83d\\ude00\""] = utf8.char(0x1F600),
  },
  REJECTED = {
    "0x10",
    "1e999",
    "-1e999",
    "01",
    "1.",
    ".5",
    "+1",
    "-",
    "1e",
    "\"a\tb\"",
    "\"a\nb\"",
    "\"\\ud800\"",
    "\"\\udc00\"",
    "\"\\ud800\\u0041\"",
    "\"\\u12\"",
    "\"\\u-123\"",
    "\"\\u 123\"",
  },
}
JsonContract.__index = JsonContract

function JsonContract.new()
  return setmetatable({ failures = {} }, JsonContract)
end

function JsonContract:expect(passed, message)
  if not passed then
    self.failures[#self.failures + 1] = message
  end
end

function JsonContract:check_round_trips()
  for _, text in ipairs(JsonContract.ROUND_TRIPS) do
    local ok, value = pcall(YonJson.decode, text)
    local encoded = ok and YonJson.encode(value) or tostring(value)
    self:expect(encoded == text, "round trip changed " .. text .. " into " .. encoded)
  end
end

function JsonContract:check_accepted()
  for text, expected in pairs(JsonContract.ACCEPTED) do
    local ok, value = pcall(YonJson.decode, text)
    self:expect(ok and value == expected, "rejected or misread valid JSON " .. text)
  end
end

function JsonContract:check_positions()
  local list = YonJson.decode("[1,null,2]")
  self:expect(#list == 3 and list[2] == YonNull and list[3] == 2, "array items move after a null")
  local first, second = YonJson.arguments(YonJson.decode("[null,5]"))
  self:expect(first == nil and second == 5, "arguments [null,5] become " .. tostring(first) .. "," .. tostring(second))
  local count = select("#", YonJson.arguments(YonJson.decode("[1,null]")))
  self:expect(count == 2, "a trailing null argument is dropped")
end

function JsonContract:check_models()
  local Item = setmetatable({}, { __name = "Item" })
  YonModels.field_types[Item] = {}
  local items = YonModels.read({ list = true, item = { model = Item } }, YonJson.decode("[null,{}]"))
  self:expect(items[1] == YonNull and getmetatable(YonNull).__name == "null", "a null list item changes the null marker")
  self:expect(getmetatable(items[2]) == Item, "a list item does not become its model")
end

function JsonContract:check_rejections()
  for _, text in ipairs(JsonContract.REJECTED) do
    self:expect(not pcall(YonJson.decode, text), "accepted invalid JSON " .. text)
  end
end

function JsonContract:run()
  self:check_round_trips()
  self:check_accepted()
  self:check_positions()
  self:check_models()
  self:check_rejections()
  if #self.failures > 0 then
    error(table.concat(self.failures, "\n"), 0)
  end
  print("Lua JSON contracts passed")
end

JsonContract.new():run()
