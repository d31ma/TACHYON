TacTasks = {
  SETTLE = "tachyon-http:settle ",
  MAX_WAITING = 64,
  OUTSIDE_TASK = "{\"error\":{\"code\":\"HTTP_REQUEST\"}}",
  TOO_MANY = "{\"error\":{\"code\":\"HTTP_CAPACITY\"}}",
  CANCELLED = { error = "The service was removed before its task finished.", code = "CANCELLED" },
  parked = {},
  waiting = {},
  waiting_count = 0,
  last_task = 0,
  last_ticket = 0,
  current = nil,
}

function TacTasks.handle(text)
  if text:sub(1, #TacTasks.SETTLE) == TacTasks.SETTLE then
    return TacTasks.settle(text)
  end
  return TacTasks.run(text)
end

function TacTasks.run(text)
  TacTasks.last_task = TacTasks.last_task + 1
  local task = { id = TacTasks.last_task, routine = coroutine.create(TacServices.answered), owner = nil, ticket = 0 }
  return TacTasks.resume(task, text)
end

function TacTasks.settle(text)
  local header, answer = text:match("^([^\n]*)\n?(.*)$")
  local task = TacTasks.stop_waiting(tonumber(header:sub(#TacTasks.SETTLE + 1)))
  if task == nil then
    return TacAnswer.value(nil)
  end
  TacTasks.parked[task.id] = nil
  task.ticket = 0
  TacServices.executing = task.owner
  local result = TacTasks.resume(task, answer)
  if coroutine.status(task.routine) == "dead" then
    TacTasks.finish(task, result)
  end
  return TacAnswer.value(nil)
end

function TacTasks.allow_waiting(owner)
  if TacTasks.current ~= nil then
    TacTasks.current.owner = owner
  end
end

function TacTasks.exchange(request)
  return TacTasks.park({ http = "start", request = request })
end

function TacTasks.park(message)
  local task = TacTasks.current
  if task == nil or task.owner == nil or coroutine.running() ~= task.routine then
    return TacTasks.OUTSIDE_TASK
  end
  if TacTasks.waiting_count >= TacTasks.MAX_WAITING then
    return TacTasks.TOO_MANY
  end
  TacTasks.last_ticket = TacTasks.last_ticket + 1
  task.ticket = TacTasks.last_ticket
  TacTasks.waiting[task.ticket] = task
  TacTasks.waiting_count = TacTasks.waiting_count + 1
  message.ticket = task.ticket
  TacWebHost.ask(message)
  return coroutine.yield()
end

function TacTasks.stop_waiting(ticket)
  local task = TacTasks.waiting[ticket]
  if task ~= nil then
    TacTasks.waiting[ticket] = nil
    TacTasks.waiting_count = TacTasks.waiting_count - 1
  end
  return task
end

function TacTasks.cancel(owner_id)
  for _, task in pairs(TacTasks.parked) do
    if owner_id == nil or task.owner.id == owner_id then
      TacTasks.drop(task)
    end
  end
end

function TacTasks.resume(task, value)
  local outer = TacTasks.current
  TacTasks.current = task
  local ok, answer = coroutine.resume(task.routine, value)
  TacTasks.current = outer
  if not ok then
    return TacAnswer.failure_of(answer)
  end
  if coroutine.status(task.routine) ~= "suspended" then
    return answer
  end
  TacTasks.parked[task.id] = task
  return { task = task.id }
end

function TacTasks.finish(task, answer)
  TacServices.drain(task.owner)
  TacWebHost.ask({ http = "done", task = task.id, answer = TacTasks.encoded(answer) })
end

function TacTasks.drop(task)
  TacTasks.parked[task.id] = nil
  if TacTasks.stop_waiting(task.ticket) ~= nil then
    TacWebHost.ask({ http = "cancel", ticket = task.ticket })
  end
  coroutine.close(task.routine)
  TacWebHost.ask({ http = "done", task = task.id, answer = TacTasks.encoded(TacTasks.CANCELLED) })
end

function TacTasks.encoded(answer)
  local ok, text = pcall(YonJson.encode, answer)
  if ok then
    return text
  end
  return YonJson.encode(TacAnswer.failure(TacAnswer.message(text)))
end
