class TacTask
  attr_reader :id, :fiber
  attr_accessor :owner, :ticket

  def initialize(id, fiber)
    @id = id
    @fiber = fiber
    @owner = nil
    @ticket = 0
  end
end

class TacTasks
  SETTLE = "tachyon-http:settle "
  MAX_WAITING = 64
  OUTSIDE_TASK = "{\"error\":{\"code\":\"HTTP_REQUEST\"}}"
  TOO_MANY = "{\"error\":{\"code\":\"HTTP_CAPACITY\"}}"
  CANCELLED = { "error" => "The service was removed before its task finished.", "code" => "CANCELLED" }

  @parked = {}
  @waiting = {}
  @last_task = 0
  @last_ticket = 0
  @current = nil

  def self.run(text)
    @last_task += 1
    task = TacTask.new(@last_task, Fiber.new { TacServices.answered(text) })
    resume(task, nil)
  end

  def self.settle(text)
    header, answer = text.split("\n", 2)
    task = @waiting.delete(header[SETTLE.size, header.size].to_i)
    return TacAnswer.value(nil) unless task
    @parked.delete(task.id)
    task.ticket = 0
    TacServices.resume_owner(task.owner)
    result = resume(task, answer.to_s)
    finish(task, result) unless task.fiber.alive?
    TacAnswer.value(nil)
  end

  def self.allow_waiting(owner)
    @current.owner = owner if @current
  end

  def self.exchange(request)
    task = @current
    return OUTSIDE_TASK if task.nil? || task.owner.nil?
    return TOO_MANY if @waiting.size >= MAX_WAITING
    @last_ticket += 1
    task.ticket = @last_ticket
    @waiting[task.ticket] = task
    TacWebHost.ask({ "http" => "start", "ticket" => task.ticket, "request" => request })
    Fiber.yield
  end

  def self.cancel(owner_id)
    @parked.values.each do |task|
      drop(task) if owner_id.nil? || task.owner.id == owner_id
    end
  end

  def self.resume(task, value)
    outer = @current
    @current = task
    begin
      answer = task.fiber.resume(value)
    ensure
      @current = outer
    end
    return answer unless task.fiber.alive?
    @parked[task.id] = task
    { "task" => task.id }
  end

  def self.finish(task, answer)
    TacServices.drain(task.owner)
    TacWebHost.ask({ "http" => "done", "task" => task.id, "answer" => JSON.generate(answer) })
  end

  def self.drop(task)
    @parked.delete(task.id)
    TacWebHost.ask({ "http" => "cancel", "ticket" => task.ticket }) if @waiting.delete(task.ticket)
    TacWebHost.ask({ "http" => "done", "task" => task.id, "answer" => JSON.generate(CANCELLED) })
  end
end
