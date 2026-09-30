BehaviorCalls = { calls = {} }

function BehaviorCalls.record(key)
  BehaviorCalls.calls[key] = true
end

function BehaviorCalls.take()
  local names = {}
  for key in pairs(BehaviorCalls.calls) do
    names[#names + 1] = key
  end
  table.sort(names)
  BehaviorCalls.calls = {}
  return names
end

Assert = {
  NOTHING_THROWN = "Expected an error to be thrown, but nothing was thrown.",
  MESSAGE_SEPARATOR = " ==> ",
}

function Assert.equals(expected, actual, message)
  Assert.check(Assert.same(expected, actual), "expected: <" .. Assert.show(expected) .. "> but was: <" .. Assert.show(actual) .. ">", message)
end

function Assert.is_true(condition, message)
  Assert.check(condition == true, "expected: <true> but was: <" .. Assert.show(condition) .. ">", message)
end

function Assert.throws(action, message)
  local ok, failure = pcall(action)
  if ok then
    error(Assert.text(Assert.NOTHING_THROWN, message), 0)
  end
  return failure
end

function Assert.same(expected, actual)
  if type(expected) ~= "table" or type(actual) ~= "table" then
    return expected == actual
  end
  for key, value in pairs(expected) do
    if not Assert.same(value, actual[key]) then
      return false
    end
  end
  for key in pairs(actual) do
    if expected[key] == nil then
      return false
    end
  end
  return true
end

function Assert.check(passed, failure, message)
  if not passed then
    error(Assert.text(failure, message), 0)
  end
end

function Assert.text(failure, message)
  if message == nil then
    return failure
  end
  return message .. Assert.MESSAGE_SEPARATOR .. failure
end

function Assert.show(value)
  if type(value) == "string" then
    return value
  end
  local ok, text = pcall(YonJson.encode, value)
  if ok then
    return text
  end
  return tostring(value)
end
