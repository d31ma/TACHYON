class ApplicationEventPublisher
  def publish_event(value, retain = false)
    name = TacValues.model_name(value)
    return TacServices.event_failure if name.nil?
    TacServices.publish(name, value, retain)
    value
  end
end

class TacAnswer
  def self.value(value)
    { "value" => TacValues.plain(value) }
  end

  def self.failure(message)
    { "error" => message.to_s }
  end

  def self.failure_of(error)
    answer = failure(error.message)
    answer["code"] = error.behavior_code if error.respond_to?(:behavior_code)
    answer
  end
end

class TacStorage
  ASK = "tachyon-storage:ask\n"
  TAKE = "tachyon-storage:take\n"
  ROOM = 4096

  def self.ask(request)
    answer = exchange(ASK + JSON.generate(request) + " " * ROOM)
    return answer unless answer.is_a?(Hash) && answer["more"].is_a?(Integer)
    exchange(TAKE + " " * answer["more"])
  end

  def self.exchange(buffer)
    TachyonWeb.host_emit(buffer)
    JSON.parse(buffer)
  end
end

class TacValues
  WRONG_SHAPE = "The server answer does not have the shape of the model (HTTP_RESPONSE)."
  PLAIN_SHAPES = { "string" => [String], "integer" => [Integer], "number" => [Integer, Float], "boolean" => [TrueClass, FalseClass] }

  @models = {}

  def self.register_models(models_json)
    @models = JSON.parse(models_json)
  end

  def self.answer(shape, value)
    return value if value.nil? || !shape.is_a?(String)
    return answer(shape[1, shape.length], value) if shape.start_with?("?") || shape.start_with?("!")
    return answer_list(shape[2, shape.length], value) if shape.start_with?("[]")
    return answer_plain(shape, value) if PLAIN_SHAPES.key?(shape)
    model = @models[shape]
    return value unless model.is_a?(Hash)
    answer_model(shape, model, value)
  end

  def self.answer_list(shape, value)
    raise wrong_shape unless value.is_a?(Array)
    value.map { |item| answer(shape, item) }
  end

  def self.answer_plain(shape, value)
    raise wrong_shape unless PLAIN_SHAPES[shape].any? { |type| value.is_a?(type) }
    value
  end

  def self.answer_model(shape, model, value)
    raise wrong_shape unless value.is_a?(Hash) && Object.const_defined?(shape)
    type = Object.const_get(shape)
    raise wrong_shape unless type.is_a?(Class) && type.respond_to?(:members)
    fields = {}
    shapes = model["fields"] || {}
    type.members.each { |member| fields[member] = answer(shapes[member.to_s], value[member.to_s]) }
    type.new(**fields)
  rescue ArgumentError
    raise wrong_shape
  end

  def self.wrong_shape
    BehaviorError.new("HTTP_RESPONSE", WRONG_SHAPE)
  end

  def self.plain(value)
    return plain_hash(value.to_h) if value.is_a?(Struct)
    return plain_hash(value.to_h) if data?(value)
    return plain_hash(value) if value.is_a?(Hash)
    return value.map { |item| plain(item) } if value.is_a?(Array)
    return value.to_s if value.is_a?(Symbol)
    value
  end

  def self.data?(value)
    Object.const_defined?(:Data) && value.is_a?(Object.const_get(:Data))
  end

  def self.model_name(value)
    return nil unless value.is_a?(Struct) || data?(value)
    value.class.name.to_s.split("::").last
  end

  def self.read(shape, value, models)
    return value if value.nil? || !shape.is_a?(String) || !models.is_a?(Hash)
    return read(shape[1, shape.length], value, models) if shape.start_with?("?") || shape.start_with?("!")
    return value.map { |item| read(shape[2, shape.length], item, models) } if shape.start_with?("[]") && value.is_a?(Array)
    model = models[shape]
    return value unless model.is_a?(Hash) && value.is_a?(Hash) && Object.const_defined?(shape)
    type = Object.const_get(shape)
    return value unless type.is_a?(Class) && type.respond_to?(:members)
    fields = {}
    type.members.each { |member| fields[member] = read(model["fields"][member.to_s], value[member.to_s], models) }
    type.new(**fields)
  rescue StandardError
    value
  end

  def self.plain_hash(value)
    plain = {}
    value.each { |key, item| plain[key.to_s] = plain(item) }
    plain
  end
end

class TacOwner
  attr_reader :route, :service, :id, :instance, :metadata, :queue

  def initialize(request, instance, metadata)
    @route = request["route"]
    @service = request["service"]
    @id = request["owner"]
    @instance = instance
    @metadata = metadata
    @queue = []
    @disposed = false
  end

  def disposed?
    @disposed
  end

  def dispose
    return if @disposed
    @disposed = true
    @queue.clear
    TacServices.names(@metadata, "stops").each { |name| @instance.public_send(name) }
  end

  def matches?(request)
    @route == request["route"] && @service == request["service"]
  end
end

class TacServices
  MAX_OWNERS = 256
  MAX_QUEUED_EVENTS = 128
  MAX_REQUEST_BYTES = 1_048_576
  MAX_MEMBER_REQUEST_BYTES = 65_536
  MAX_EVENT_BYTES = 65_536
  HEXADECIMAL = "0123456789abcdefABCDEF"
  HYPHENS = [8, 13, 18, 23]

  @services = {}
  @owners = {}
  @executing = nil
  @handling = false

  def self.register(name, service_class, metadata_json)
    @services[name] = [service_class, JSON.parse(metadata_json)]
  end

  def self.names(metadata, key)
    metadata[key] || []
  end

  def self.ruby_name(metadata, name)
    (metadata["names"] || {})[name] || name
  end

  def self.invoke(text)
    return JSON.generate(TacAnswer.failure("Native owner is busy.")) if @handling
    @handling = true
    begin
      answer = text.start_with?(TacTasks::SETTLE) ? TacTasks.settle(text) : TacTasks.run(text)
      drain(@executing) if @executing
      JSON.generate(answer)
    rescue StandardError => error
      JSON.generate(TacAnswer.failure(error.message))
    ensure
      @executing = nil
      @handling = false
    end
  end

  def self.answered(text)
    handle(text)
  rescue StandardError => error
    TacAnswer.failure_of(error)
  end

  def self.resume_owner(owner)
    @executing = owner
  end

  def self.handle(text)
    return too_large("request", text.bytesize, MAX_REQUEST_BYTES) if text.bytesize > MAX_REQUEST_BYTES
    request = read(text)
    return TacAnswer.failure("Invalid companion request.") if request.nil?
    operation = request["op"]
    if operation != "init" && text.bytesize > MAX_MEMBER_REQUEST_BYTES
      return too_large("request to #{request['name'] || operation}", text.bytesize, MAX_MEMBER_REQUEST_BYTES)
    end
    return TacAnswer.failure("Companion request has no operation.") unless operation.is_a?(String) && !operation.empty?
    if operation == "host-dispose"
      dispose_all
      return TacAnswer.value(nil)
    end
    problem = identity_problem(request)
    return problem if problem
    handle_owned(request, operation)
  end

  def self.read(text)
    request = JSON.parse(text)
    request.is_a?(Hash) ? request : nil
  rescue StandardError
    nil
  end

  def self.too_large(subject, size, limit)
    TacAnswer.failure("The #{subject} is #{size} bytes. The limit is #{limit} bytes. Pass less data to the service.")
  end

  def self.identity_problem(request)
    return { "error" => "Missing companion route.", "code" => "TY_NATIVE_ROUTE" } unless present?(request["route"])
    return { "error" => "Missing companion name.", "code" => "TY_NATIVE_SERVICE" } unless present?(request["service"])
    return { "error" => "Missing native owner identity.", "code" => "TY_NATIVE_OWNER" } unless owner_id?(request["owner"])
    nil
  end

  def self.present?(value)
    value.is_a?(String) && !value.empty?
  end

  def self.owner_id?(text)
    return false unless text.is_a?(String) && text.size == 36
    text.chars.each_with_index do |character, index|
      if HYPHENS.include?(index)
        return false unless character == "-"
      else
        return false unless HEXADECIMAL.include?(character)
      end
    end
    true
  end

  def self.handle_owned(request, operation)
    if operation == "init"
      failed = activate(request)
      return failed if failed
    end
    owner = @owners[request["owner"]]
    return { "error" => "Stale native owner.", "code" => "TY_NATIVE_OWNER" } unless owner && owner.matches?(request)
    if operation == "dispose"
      dispose_owner(owner.id)
      return TacAnswer.value(nil)
    end
    @executing = owner
    return TacAnswer.value(describe(owner.metadata)) if operation == "init"
    name = request["name"]
    return TacAnswer.failure("Companion request has no member name.") unless present?(name)
    TacTasks.allow_waiting(owner) if operation == "call" || operation == "event"
    return receive(owner, name, request["value"]) if operation == "event"
    use_member(owner, operation, name, request)
  end

  def self.activate(request)
    existing = @owners[request["owner"]]
    return nil if existing && existing.matches?(request)
    dispose_owner(request["owner"]) if existing
    registered = @services[request["service"]]
    return { "error" => "Unknown native service.", "code" => "TY_NATIVE_SERVICE" } unless registered
    return TacAnswer.failure("Too many native activations.") if @owners.size >= MAX_OWNERS
    service_class, metadata = registered
    owner = TacOwner.new(request, service_class.new, metadata)
    @owners[owner.id] = owner
    @executing = owner
    assign_properties(owner, request["props"])
    start(owner)
  end

  def self.start(owner)
    names(owner.metadata, "starts").each { |name| owner.instance.public_send(name) }
    drain(owner)
    nil
  rescue StandardError
    dispose_owner(owner.id)
    TacAnswer.failure("Native initializer failed.")
  end

  def self.assign_properties(owner, properties)
    return unless properties.is_a?(Hash)
    required = names(owner.metadata, "required")
    fields = names(owner.metadata, "fields")
    properties.each do |key, value|
      field = ruby_name(owner.metadata, key)
      next unless required.include?(key) || required.include?(field) || required.include?("*")
      next unless fields.include?(key)
      owner.instance.instance_variable_set("@#{field}", value)
    end
  end

  def self.describe(metadata)
    listeners = names(metadata, "listeners").map do |listener|
      {
        "key" => listener["method"],
        "events" => listener["events"],
        "replay" => listener["replay"],
        "order" => listener["order"],
        "asynchronous" => listener["asynchronous"],
        "condition" => listener["condition"],
        "models" => listener["models"]
      }
    end
    { "fields" => names(metadata, "fields"), "methods" => names(metadata, "methods"), "listeners" => listeners }
  end

  def self.receive(owner, name, value)
    listener = names(owner.metadata, "listeners").find { |candidate| candidate["method"] == name }
    return TacAnswer.failure("Unknown companion listener: #{name}") unless listener
    method = owner.instance.method(name)
    payload = TacValues.read(listener["events"].first, value, listener["models"])
    result = method.arity == 0 ? owner.instance.public_send(name) : owner.instance.public_send(name, payload)
    republish(result, listener["returns"]) if result && !listener["asynchronous"]
    TacAnswer.value(nil)
  rescue StandardError
    TacAnswer.failure("Native event listener failed.")
  end

  def self.use_member(owner, operation, name, request)
    fields = names(owner.metadata, "fields")
    return TacAnswer.value(owner.instance.public_send(ruby_name(owner.metadata, name))) if operation == "get" && fields.include?(name)
    return write(owner, name, request["value"]) if operation == "set" && fields.include?(name)
    return call(owner, name, request["args"]) if operation == "call" && names(owner.metadata, "methods").include?(name)
    return TacAnswer.failure("Unknown companion member: #{name}") unless fields.include?(name) || names(owner.metadata, "methods").include?(name)
    TacAnswer.failure("Companion member does not support #{operation}: #{name}")
  end

  def self.write(owner, name, value)
    return TacAnswer.failure("Companion field is read-only: #{name}") unless names(owner.metadata, "writable").include?(name)
    readers = owner.metadata["readers"] || {}
    field = TacValues.read((readers["fields"] || {})[name], value, readers["models"])
    owner.instance.public_send("#{ruby_name(owner.metadata, name)}=", field)
    TacAnswer.value(nil)
  end

  def self.call(owner, name, arguments)
    arguments = [] unless arguments.is_a?(Array)
    readers = owner.metadata["readers"] || {}
    shapes = (readers["parameters"] || {})[name] || []
    given = []
    arguments.each_with_index { |argument, index| given << TacValues.read(shapes[index], argument, readers["models"]) }
    TacAnswer.value(owner.instance.public_send(ruby_name(owner.metadata, name), *given))
  end

  def self.publish(name, value, retain)
    owner = @executing
    return event_failure if owner.nil? || owner.disposed?
    encoded = JSON.generate(TacValues.plain(value))
    return event_failure if encoded.bytesize > MAX_EVENT_BYTES || owner.queue.size >= MAX_QUEUED_EVENTS
    owner.queue << [name.to_s, encoded, retain == true]
  rescue StandardError
    event_failure
  end

  def self.republish(result, returns)
    return unless returns.is_a?(String)
    items = result.is_a?(Array) ? result.first(32) : [result]
    items.each { |item| publish(returns, item, false) unless item.nil? }
  end

  def self.drain(owner)
    until owner.disposed? || owner.queue.empty?
      name, encoded, retain = owner.queue.shift
      send_event(owner, name, encoded, retain)
    end
  end

  def self.send_event(owner, name, encoded, retain)
    return unless TachyonWeb.respond_to?(:host_emit)
    payload = "{\"route\":#{JSON.generate(owner.route)},\"service\":#{JSON.generate(owner.service)},\"owner\":#{JSON.generate(owner.id)},\"name\":#{JSON.generate(name)},\"value\":#{encoded},\"retain\":#{retain}}"
    TachyonWeb.host_emit(payload)
  end

  def self.event_failure
    STDERR.write("Tachyon native event delivery failed.\n")
    nil
  end

  def self.dispose_owner(id)
    TacTasks.cancel(id)
    owner = @owners.delete(id)
    @executing = nil if @executing && @executing.id == id
    owner&.dispose
  rescue StandardError
    event_failure
  end

  def self.dispose_all
    TacTasks.cancel(nil)
    active = @owners.values
    @owners = {}
    @executing = nil
    active.each(&:dispose)
  end
end

module TachyonWeb
  def self.invoke(text)
    TacServices.invoke(text)
  end
end
