ClockContract = {
  TIMEOUT_MS = 500,
  BLOCKING_CALL = "sleep 3",
  WAIT_MS = 1200,
  MOST_CPU_SECONDS = 0.25,
}
ClockContract.__index = ClockContract

function ClockContract.new()
  Behaviors.register("{}", "yon", string.rep("a", 64))
  return setmetatable({ failures = {}, attempts = 0 }, ClockContract)
end

function ClockContract:expect(passed, message)
  if not passed then
    self.failures[#self.failures + 1] = message
  end
end

function ClockContract:check_sleep_commands()
  self:expect(BehaviorClock.sleep_command(1500, "/") == "sleep 1.500", "POSIX sleep command")
  self:expect(BehaviorClock.sleep_command(7, "/") == "sleep 0.007", "short POSIX sleep command")
  self:expect(
    BehaviorClock.sleep_command(250, "\\") == "powershell -NoProfile -NonInteractive -Command Start-Sleep -Milliseconds 250",
    "Windows sleep command"
  )
end

function ClockContract:check_wait_uses_no_cpu()
  local cpu = os.clock()
  local started = os.time()
  local before = BehaviorClock.monotonic_ms()
  BehaviorClock.wait(ClockContract.WAIT_MS)
  local used = os.clock() - cpu
  self:expect(used < ClockContract.MOST_CPU_SECONDS, "a wait used " .. used .. " s of CPU")
  self:expect(os.difftime(os.time(), started) >= 1, "a wait returned before its time")
  self:expect(BehaviorClock.monotonic_ms() - before >= ClockContract.WAIT_MS, "the clock missed the time spent waiting")
end

function ClockContract:check_timeout_during_blocking_call()
  local record = { name = "Orders", methods = {
    slow = { retryable = { maxRetries = 3, delayMs = 0, multiplier = 1, maxDelayMs = 100, jitterMs = 0 } },
  } }
  local context = BehaviorContext.new({ Orders = record }, {
    partition = "tenant",
    signal = BehaviorSignal.new(),
    timeoutMs = ClockContract.TIMEOUT_MS,
  })
  context:configure(function()
    return {}
  end)
  local ok, failure = pcall(context.invoke, context, { behaviors = context }, "Orders", "slow", table.pack(), function()
    self.attempts = self.attempts + 1
    os.execute(ClockContract.BLOCKING_CALL)
    error("transient")
  end)
  local code = BehaviorError.is(failure) and failure.behavior_code or tostring(failure)
  self:expect(not ok and code == "timeout", "a timeout during a blocking call did not fire: " .. code)
  self:expect(self.attempts == 1, "a timed-out call was retried " .. self.attempts .. " times")
end

function ClockContract:run()
  self:check_sleep_commands()
  self:check_wait_uses_no_cpu()
  self:check_timeout_during_blocking_call()
  if #self.failures > 0 then
    error(table.concat(self.failures, "\n"), 0)
  end
  print("Lua behavior clock contracts passed")
end

ClockContract.new():run()
