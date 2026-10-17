
BrowserFakes = {
  MISSING = "This browser API needs the page. A test unit has no page: give the call an answer with BrowserFakes.answer in the test unit.",
  answers = {},
}

function BrowserFakes.answer(api, call, value)
  BrowserFakes.answers[api .. "." .. call] = { value = value }
end

function BrowserFakes.reply(request)
  local found = BrowserFakes.answers[tostring(request.api) .. "." .. tostring(request.call)]
  if found == nil then
    return { error = BrowserFakes.MISSING }
  end
  return { value = found.value }
end

TacWebHost = {}

function TacWebHost.call(api, call, arguments)
  local answer = TacWebHost.ask({ api = api, call = call, args = arguments or {} })
  if type(answer.error) == "string" then
    error(answer.error, 0)
  end
  return answer.value
end

function TacWebHost.ask(request)
  local ok, answer = pcall(YonJson.decode, TachyonHost.ask(YonJson.encode(request)) or "")
  if not ok or type(answer) ~= "table" then
    return { error = "Tachyon browser APIs did not answer." }
  end
  return answer
end

Clock = {}

function Clock:now_ms()
  return TacWebHost.call("clock", "now") + 0.0
end

function Clock:monotonic_ms()
  return TacWebHost.call("clock", "monotonic") + 0.0
end

function Clock:time_zone()
  return tostring(TacWebHost.call("clock", "timeZone"))
end

BehaviorHostClock = {
  NO_WAIT = "Lua services in the browser cannot wait between attempts. Write this service in another Tac language, or leave Retry out.",
}

function BehaviorHostClock.now_ms()
  return math.floor(Clock:now_ms())
end

function BehaviorHostClock.monotonic_ms()
  return Clock:monotonic_ms()
end

function BehaviorHostClock.wait()
  error(BehaviorHostClock.NO_WAIT, 0)
end

SecureRandom = {}

function SecureRandom:bytes(count)
  return TacWebHost.call("secureRandom", "bytes", { count })
end

function SecureRandom:uuid()
  return tostring(TacWebHost.call("secureRandom", "uuid"))
end

Connectivity = {}

function Connectivity:online()
  return TacWebHost.call("connectivity", "online") == true
end

PageVisibility = {}

function PageVisibility:visible()
  return TacWebHost.call("pageVisibility", "visible") == true
end

Preferences = {}

function Preferences:scheme()
  return tostring(TacWebHost.call("preferences", "scheme"))
end

function Preferences:motion()
  return tostring(TacWebHost.call("preferences", "motion"))
end

function Preferences:language()
  return tostring(TacWebHost.call("preferences", "language"))
end

Clipboard = {}

function Clipboard:write_text(text)
  TacWebHost.call("clipboard", "writeText", { text })
end

function Clipboard:read_text()
  TacWebHost.call("clipboard", "readText")
end

Share = {}

function Share:open(title, text, url)
  TacWebHost.call("share", "open", { title, text, url })
end

Notifications = {}

function Notifications:permission()
  return tostring(TacWebHost.call("notifications", "permission"))
end

function Notifications:request_permission()
  TacWebHost.call("notifications", "requestPermission")
end

function Notifications:show(title, body)
  TacWebHost.call("notifications", "show", { title, body })
end

Geolocation = {}

function Geolocation:current_position()
  TacWebHost.call("geolocation", "currentPosition")
end
