YonShapeError = {}
YonShapeError.__index = YonShapeError

YonNoRoute = {}
YonNoRoute.__index = YonNoRoute

YonNull = setmetatable({}, { __name = "null" })

YonJson = {
  MAX_DEPTH = 64,
  NOT_SERIALIZABLE = "Handler returned a value that is not JSON-serializable.",
  OBJECT = {},
  ESCAPES = { ['"'] = '\\"', ["\\"] = "\\\\", ["\b"] = "\\b", ["\f"] = "\\f", ["\n"] = "\\n", ["\r"] = "\\r", ["\t"] = "\\t" },
  UNESCAPES = { ['"'] = '"', ["\\"] = "\\", ["/"] = "/", b = "\b", f = "\f", n = "\n", r = "\r", t = "\t" },
  CONTROL_CHARACTER = "[%z\1-\31]",
  FOUR_HEX_DIGITS = "^%x%x%x%x$",
}

function YonJson.shape_error(message)
  error(setmetatable({ message = message }, YonShapeError), 0)
end

function YonJson.encode(value)
  local parts = {}
  YonJson.write(parts, value, 0)
  return table.concat(parts)
end

function YonJson.write(parts, value, depth)
  if depth > YonJson.MAX_DEPTH then
    YonJson.shape_error(YonJson.NOT_SERIALIZABLE)
  end
  local kind = type(value)
  if value == nil or value == YonNull then
    parts[#parts + 1] = "null"
  elseif kind == "boolean" then
    parts[#parts + 1] = tostring(value)
  elseif kind == "number" then
    parts[#parts + 1] = YonJson.number(value)
  elseif kind == "string" then
    parts[#parts + 1] = YonJson.text(value)
  elseif kind == "table" then
    YonJson.write_table(parts, value, depth)
  else
    YonJson.shape_error(YonJson.NOT_SERIALIZABLE)
  end
end

function YonJson.number(value)
  if math.type(value) == "integer" then
    return string.format("%d", value)
  end
  if value ~= value or value == math.huge or value == -math.huge then
    YonJson.shape_error(YonJson.NOT_SERIALIZABLE)
  end
  if value == math.floor(value) and math.abs(value) < 1e15 then
    return string.format("%.1f", value)
  end
  return string.format("%.17g", value)
end

function YonJson.text(value)
  if utf8.len(value) == nil then
    YonJson.shape_error(YonJson.NOT_SERIALIZABLE)
  end
  local escaped = value:gsub('[%c"\\]', function(character)
    return YonJson.ESCAPES[character] or string.format("\\u%04x", character:byte())
  end)
  return '"' .. escaped .. '"'
end

function YonJson.write_table(parts, value, depth)
  local fields = YonModels.fields_of(value)
  if fields ~= nil then
    YonJson.write_fields(parts, value, fields, depth)
  elseif getmetatable(value) ~= YonJson.OBJECT and YonJson.is_list(value) then
    YonJson.write_list(parts, value, depth)
  else
    YonJson.write_fields(parts, value, YonJson.sorted_keys(value), depth)
  end
end

function YonJson.is_list(value)
  local count = 0
  for key in pairs(value) do
    if math.type(key) ~= "integer" or key < 1 then
      return false
    end
    count = count + 1
  end
  return count == #value
end

function YonJson.sorted_keys(value)
  local keys = {}
  for key in pairs(value) do
    if type(key) ~= "string" then
      YonJson.shape_error(YonJson.NOT_SERIALIZABLE)
    end
    keys[#keys + 1] = key
  end
  table.sort(keys)
  return keys
end

function YonJson.write_list(parts, value, depth)
  parts[#parts + 1] = "["
  for index = 1, #value do
    if index > 1 then
      parts[#parts + 1] = ","
    end
    YonJson.write(parts, value[index], depth + 1)
  end
  parts[#parts + 1] = "]"
end

function YonJson.write_fields(parts, value, keys, depth)
  parts[#parts + 1] = "{"
  for index, key in ipairs(keys) do
    if index > 1 then
      parts[#parts + 1] = ","
    end
    parts[#parts + 1] = YonJson.text(key)
    parts[#parts + 1] = ":"
    YonJson.write(parts, value[key], depth + 1)
  end
  parts[#parts + 1] = "}"
end

function YonJson.decode(text)
  local reader = { text = text, at = 1 }
  local value = YonJson.read(reader, 0)
  YonJson.skip(reader)
  if reader.at <= #text then
    YonJson.fail(reader)
  end
  return value
end

function YonJson.fail(reader)
  error("Invalid JSON at byte " .. reader.at, 0)
end

function YonJson.skip(reader)
  reader.at = reader.text:find("[^ \t\r\n]", reader.at) or (#reader.text + 1)
end

function YonJson.read(reader, depth)
  if depth > YonJson.MAX_DEPTH then
    YonJson.fail(reader)
  end
  YonJson.skip(reader)
  local character = reader.text:sub(reader.at, reader.at)
  if character == "{" then
    return YonJson.read_object(reader, depth)
  elseif character == "[" then
    return YonJson.read_array(reader, depth)
  elseif character == '"' then
    return YonJson.read_text(reader)
  end
  return YonJson.read_word(reader)
end

function YonJson.read_word(reader)
  local word = reader.text:match("^[%w%.%+%-]+", reader.at)
  if word == nil then
    YonJson.fail(reader)
  end
  reader.at = reader.at + #word
  if word == "true" then
    return true
  elseif word == "false" then
    return false
  elseif word == "null" then
    return nil
  end
  local number = YonJson.is_number(word) and tonumber(word) or nil
  if number == nil or number == math.huge or number == -math.huge then
    YonJson.fail(reader)
  end
  return number
end

function YonJson.is_number(word)
  local whole, rest = word:match("^%-?(%d+)(.*)$")
  if whole == nil or (#whole > 1 and whole:sub(1, 1) == "0") then
    return false
  end
  rest = rest:match("^%.%d+(.*)$") or rest
  rest = rest:match("^[eE][%+%-]?%d+(.*)$") or rest
  return rest == ""
end

function YonJson.read_text(reader)
  local parts = {}
  local at = reader.at + 1
  while true do
    local stop = reader.text:find('["\\]', at)
    if stop == nil then
      YonJson.fail(reader)
    end
    parts[#parts + 1] = reader.text:sub(at, stop - 1)
    if parts[#parts]:find(YonJson.CONTROL_CHARACTER) then
      YonJson.fail(reader)
    end
    if reader.text:sub(stop, stop) == '"' then
      reader.at = stop + 1
      return table.concat(parts)
    end
    at = YonJson.read_escape(reader, parts, stop + 1)
  end
end

function YonJson.read_escape(reader, parts, at)
  local letter = reader.text:sub(at, at)
  if letter ~= "u" then
    parts[#parts + 1] = YonJson.UNESCAPES[letter] or YonJson.fail(reader)
    return at + 1
  end
  local code = YonJson.read_code(reader, at + 1)
  local after = at + 5
  if code >= 0xDC00 and code <= 0xDFFF then
    YonJson.fail(reader)
  end
  if code >= 0xD800 and code <= 0xDBFF then
    if reader.text:sub(after, after + 1) ~= "\\u" then
      YonJson.fail(reader)
    end
    local low = YonJson.read_code(reader, after + 2)
    if low < 0xDC00 or low > 0xDFFF then
      YonJson.fail(reader)
    end
    code = 0x10000 + (code - 0xD800) * 0x400 + (low - 0xDC00)
    after = after + 6
  end
  parts[#parts + 1] = utf8.char(code)
  return after
end

function YonJson.read_code(reader, at)
  local digits = reader.text:sub(at, at + 3)
  if not digits:match(YonJson.FOUR_HEX_DIGITS) then
    YonJson.fail(reader)
  end
  return tonumber(digits, 16)
end

function YonJson.read_array(reader, depth)
  local list = {}
  reader.at = reader.at + 1
  YonJson.skip(reader)
  if reader.text:sub(reader.at, reader.at) == "]" then
    reader.at = reader.at + 1
    return list
  end
  while true do
    local item = YonJson.read(reader, depth + 1)
    if item == nil then
      item = YonNull
    end
    list[#list + 1] = item
    YonJson.skip(reader)
    local character = reader.text:sub(reader.at, reader.at)
    reader.at = reader.at + 1
    if character == "]" then
      return list
    elseif character ~= "," then
      YonJson.fail(reader)
    end
  end
end

function YonJson.read_object(reader, depth)
  local object = setmetatable({}, YonJson.OBJECT)
  reader.at = reader.at + 1
  YonJson.skip(reader)
  if reader.text:sub(reader.at, reader.at) == "}" then
    reader.at = reader.at + 1
    return object
  end
  while true do
    YonJson.skip(reader)
    if reader.text:sub(reader.at, reader.at) ~= '"' then
      YonJson.fail(reader)
    end
    local key = YonJson.read_text(reader)
    YonJson.skip(reader)
    if reader.text:sub(reader.at, reader.at) ~= ":" then
      YonJson.fail(reader)
    end
    reader.at = reader.at + 1
    object[key] = YonJson.read(reader, depth + 1)
    YonJson.skip(reader)
    local character = reader.text:sub(reader.at, reader.at)
    reader.at = reader.at + 1
    if character == "}" then
      return object
    elseif character ~= "," then
      YonJson.fail(reader)
    end
  end
end

function YonJson.arguments(list)
  local values = {}
  for index = 1, #list do
    if list[index] ~= YonNull then
      values[index] = list[index]
    end
  end
  return table.unpack(values, 1, #list)
end

YonModels = {
  field_order = {},
  field_types = {},
}

function YonModels.fields_of(value)
  local model = getmetatable(value)
  if model == nil then
    return nil
  end
  return YonModels.field_order[model]
end

function YonModels.read(descriptor, value)
  if type(value) ~= "table" or descriptor == nil or value == YonNull then
    return value
  end
  if descriptor.list then
    for index = 1, #value do
      value[index] = YonModels.read(descriptor.item, value[index])
    end
    return value
  end
  local model = descriptor.model
  for name, field in pairs(YonModels.field_types[model] or {}) do
    value[name] = YonModels.read(field, value[name])
  end
  return setmetatable(value, model)
end
