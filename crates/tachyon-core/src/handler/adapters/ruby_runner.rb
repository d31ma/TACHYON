# frozen_string_literal: true

require "json"
require "net/http"
require "uri"

class YonServiceLimits
  MAX_RELAY_STDOUT_BYTES = 16 * 1024 * 1024
  MAX_RELAY_STDERR_BYTES = 64 * 1024
  MAX_RESOURCE_BYTES = 1024 * 1024
  MAX_RESOURCE_HEAD_BYTES = 8192
  RESOURCE_TIMEOUT_SECONDS = 5
  GATEWAY_ANSWER_MARGIN_SECONDS = 1
  CHUNK_BYTES = 65_536
  RELAY_POLL_SECONDS = 0.002
  freeze
end

class YonClock
  READ = Process.method(:clock_gettime)
  MONOTONIC = Process::CLOCK_MONOTONIC

  def self.now
    READ.call(MONOTONIC)
  end

  def self.time_left(deadline)
    deadline - now
  end

  freeze
end

class YonResourceError < StandardError
  MESSAGE = "Yon managed resource operation failed."

  def initialize(_message = nil)
    super(MESSAGE)
  end

  freeze
end

class YonResourceSettings
  ENDPOINT = %r{\Ahttp://127\.0\.0\.1:([0-9]{1,5})/\z}
  TOKEN = /\A[a-f0-9]{64}\z/
  HIGHEST_PORT = 65_535

  def self.port
    match = ENDPOINT.match(ENV.fetch("YON_RESOURCE_ENDPOINT", ""))
    raise YonResourceError unless match

    number = Integer(match[1], 10)
    raise YonResourceError unless number.between?(1, HIGHEST_PORT)

    number
  end

  def self.token
    token = ENV.fetch("YON_RESOURCE_TOKEN", "")
    raise YonResourceError unless TOKEN.match?(token)

    token
  end

  freeze
end

class YonHttpAnswer
  STATUS_LINE_START = "HTTP/1.1 200 "
  HEAD_END = "\r\n\r\n"
  DIGITS = /\A[0-9]+\z/

  def self.body(answer)
    split = answer.index(HEAD_END)
    raise YonResourceError if split.nil? || split > YonServiceLimits::MAX_RESOURCE_HEAD_BYTES

    head = utf8(answer.byteslice(0, split))
    body = utf8(answer.byteslice(split + HEAD_END.bytesize, answer.bytesize))
    raise YonResourceError unless head.start_with?(STATUS_LINE_START)
    raise YonResourceError if body.bytesize > YonServiceLimits::MAX_RESOURCE_BYTES
    raise YonResourceError unless content_length(head) == body.bytesize

    body
  end

  def self.utf8(bytes)
    text = bytes.dup.force_encoding(Encoding::UTF_8)
    raise YonResourceError unless text.valid_encoding?

    text
  end

  def self.content_length(head)
    lengths = head.split("\r\n").drop(1).filter_map { |line| header_length(line) }
    raise YonResourceError unless lengths.size == 1

    lengths.first
  end

  def self.header_length(line)
    name, separator, value = line.partition(":")
    raise YonResourceError if separator.empty?

    name = name.downcase
    value = value.strip
    raise YonResourceError if name == "transfer-encoding"
    raise YonResourceError if name == "content-encoding" && value.downcase != "identity"
    return nil unless name == "content-length"
    raise YonResourceError unless DIGITS.match?(value)

    Integer(value, 10)
  end

  freeze
end

class YonResourceExchange
  HOST = "127.0.0.1"
  LONGEST_ANSWER = YonServiceLimits::MAX_RESOURCE_HEAD_BYTES + YonServiceLimits::MAX_RESOURCE_BYTES + 4 + 1

  def self.encode(command)
    body = JSON.generate(command, strict: true)
    raise YonResourceError if body.bytesize > YonServiceLimits::MAX_RESOURCE_BYTES

    body
  end

  def self.post(body, timeout_seconds)
    port = YonResourceSettings.port
    token = YonResourceSettings.token
    raise YonResourceError if body.bytesize > YonServiceLimits::MAX_RESOURCE_BYTES

    deadline = YonClock.now + timeout_seconds
    socket = connect(port, timeout_seconds)
    begin
      send_all(socket, request_text(port, token, body), deadline)
      JSON.parse(YonHttpAnswer.body(receive_all(socket, deadline)))
    ensure
      socket.close
    end
  end

  def self.connect(port, timeout_seconds)
    require "socket"
    TCPSocket.new(HOST, port, connect_timeout: timeout_seconds)
  end

  def self.request_text(port, token, body)
    "POST / HTTP/1.1\r\n" \
      "Host: #{HOST}:#{port}\r\n" \
      "Authorization: Bearer #{token}\r\n" \
      "Content-Type: application/json\r\n" \
      "Content-Length: #{body.bytesize}\r\n" \
      "Accept-Encoding: identity\r\n" \
      "Connection: close\r\n\r\n#{body}"
  end

  def self.send_all(socket, text, deadline)
    offset = 0
    while offset < text.bytesize
      written = socket.write_nonblock(text.byteslice(offset, YonServiceLimits::CHUNK_BYTES), exception: false)
      if written == :wait_writable
        wait_until(deadline) { |seconds| socket.wait_writable(seconds) }
        next
      end
      offset += written
    end
  end

  def self.receive_all(socket, deadline)
    answer = String.new(encoding: Encoding::BINARY)
    loop do
      chunk = socket.read_nonblock(YonServiceLimits::CHUNK_BYTES, exception: false)
      return answer if chunk.nil?

      if chunk == :wait_readable
        wait_until(deadline) { |seconds| socket.wait_readable(seconds) }
        next
      end
      answer << chunk
      raise YonResourceError if answer.bytesize > LONGEST_ANSWER
    end
  end

  def self.wait_until(deadline)
    seconds = YonClock.time_left(deadline)
    raise YonResourceError if seconds <= 0
    raise YonResourceError unless yield(seconds)
  end

  freeze
end

class YonResources
  def self.call(command)
    body = YonResourceExchange.encode(command)
    YonResourceExchange.post(body, YonServiceLimits::RESOURCE_TIMEOUT_SECONDS)
  rescue StandardError
    raise YonResourceError, cause: nil
  end

  freeze
end

class DataSource
  def command(command)
    YonResources.call(command)
  end

  def query(sql, parameters = [])
    YonResources.call({ "operation" => "sql", "sql" => sql, "parameters" => Array(parameters), "mode" => "rows" })
  end

  def execute(sql, parameters = [])
    return YonResources.call(sql) unless sql.is_a?(String)

    YonResources.call({ "operation" => "sql", "sql" => sql, "parameters" => Array(parameters), "mode" => "execute" })
  end
end

class RedisTemplate
  def get(key)
    YonResources.call({ "operation" => "redis", "command" => "get", "key" => key })
  end

  def set(key, value, ttl_seconds)
    YonResources.call({ "operation" => "redis", "command" => "set", "key" => key, "value" => value, "ttlSeconds" => ttl_seconds })
  end

  def delete(key)
    YonResources.call({ "operation" => "redis", "command" => "delete", "key" => key })
  end
end

class YonGatewayClient
  attr_reader :deadline

  def initialize(deadline)
    @deadline = Float(deadline)
    freeze
  end

  def call(key, arguments)
    body = request_body(key, arguments)
    return failure("invalid") if body.nil?
    return failure("timeout") if YonClock.time_left(deadline) <= 0

    begin
      answer = exchange(body)
    rescue StandardError
      return exchange_failure
    end
    return failure("adapter") unless valid_answer?(answer)

    answer
  end

  private

  def request_body(key, arguments)
    return nil unless arguments.is_a?(Enumerable)

    YonResourceExchange.encode({ "operation" => "gateway", "key" => key, "arguments" => arguments.to_a })
  rescue StandardError
    nil
  end

  def exchange(body)
    timeout = YonClock.time_left(deadline) + YonServiceLimits::GATEWAY_ANSWER_MARGIN_SECONDS
    YonResourceExchange.post(body, timeout)
  end

  def exchange_failure
    return failure("timeout") if YonClock.time_left(deadline) <= 0

    failure("adapter")
  end

  def valid_answer?(answer)
    return false unless answer.is_a?(Hash)

    answer.key?("result") || answer["error"].is_a?(String)
  end

  def failure(kind)
    { "error" => kind }
  end

  freeze
end

class YonRelayOutput
  attr_reader :kept

  def initialize(reader, limit)
    @reader = reader
    @limit = limit
    @kept = String.new(encoding: Encoding::BINARY)
    @overflowed = false
    @thread = Thread.new { collect }
  end

  def overflowed?
    @overflowed
  end

  def finished_by?(deadline)
    seconds = YonClock.time_left(deadline)
    return false if seconds <= 0

    !@thread.join(seconds).nil?
  end

  def collect
    loop { keep(@reader.readpartial(YonServiceLimits::CHUNK_BYTES)) }
  rescue EOFError, IOError, SystemCallError
    nil
  ensure
    close
  end

  def keep(chunk)
    room = @limit - @kept.bytesize
    @overflowed = true if chunk.bytesize > room
    @kept << chunk.byteslice(0, room) if room.positive?
  end

  def close
    @reader.close unless @reader.closed?
  rescue IOError
    nil
  end

  freeze
end

class YonRelayInput
  def initialize(writer, payload)
    @writer = writer
    @thread = Thread.new { send_payload(payload) }
  end

  def send_payload(payload)
    @writer.write(payload)
  rescue IOError, SystemCallError
    nil
  ensure
    close
  end

  def close
    @writer.close unless @writer.closed?
  rescue IOError
    nil
  end

  freeze
end

class YonRelayChild
  def self.start(command, environment, payload)
    input = IO.pipe
    output = IO.pipe
    errors = IO.pipe
    pid = spawn_program(command, environment, input[0], output[1], errors[1])
    [input[0], output[1], errors[1]].each(&:close)
    new(pid, YonRelayInput.new(input[1], payload), output[0], errors[0])
  rescue SystemCallError, IOError, ArgumentError, TypeError
    [input, output, errors].compact.flatten.each { |pipe| pipe.close unless pipe.closed? }
    nil
  end

  def self.spawn_program(command, environment, input, output, errors)
    program = command.first
    Process.spawn(environment, [program, program], *command.drop(1),
                  unsetenv_others: true, close_others: true, in: input, out: output, err: errors)
  end

  def initialize(pid, input, output, errors)
    @pid = pid
    @input = input
    @stdout = YonRelayOutput.new(output, YonServiceLimits::MAX_RELAY_STDOUT_BYTES)
    @stderr = YonRelayOutput.new(errors, YonServiceLimits::MAX_RELAY_STDERR_BYTES)
    @status = nil
  end

  def stdout
    @stdout.kept
  end

  def overflowed?
    @stdout.overflowed? || @stderr.overflowed?
  end

  def finish(deadline)
    return nil unless exited_by?(deadline)
    return nil unless @stdout.finished_by?(deadline) && @stderr.finished_by?(deadline)

    @status
  end

  def exited_by?(deadline)
    loop do
      return true if reaped?

      seconds = YonClock.time_left(deadline)
      return false if seconds <= 0

      sleep([seconds, YonServiceLimits::RELAY_POLL_SECONDS].min)
    end
  end

  def reaped?
    _pid, status = Process.wait2(@pid, Process::WNOHANG)
    @status = status if status
    !@status.nil?
  rescue Errno::ECHILD
    @status = :lost
    true
  end

  def stop
    @input.close
    @stdout.close
    @stderr.close
    return if @status

    kill
    Process.wait2(@pid)
    @status = :killed
  rescue Errno::ECHILD
    @status = :lost
  end

  def kill
    Process.kill(:KILL, @pid)
  rescue Errno::ESRCH
    nil
  end

  freeze
end

class YonRelayProgram
  EVENTS = STDERR
  FAILURE_BODY = "{\"error\": \"Component invocation failed.\"}"

  attr_reader :request_id, :deadline, :environment

  def self.environment(environment)
    kept = {}
    environment.each do |name, value|
      upper = name.to_s.upcase
      next if upper == "YON_APPLICATION_JSON" || upper.start_with?("YON_RESOURCE_")

      kept[name.to_s] = value.to_s
    end
    kept
  end

  def self.frozen_environment(environment)
    frozen = {}
    environment.each { |name, value| frozen[name.to_s.dup.freeze] = value.to_s.dup.freeze }
    frozen.freeze
  end

  def initialize(request_id:, deadline:, environment:)
    @request_id = String(request_id).dup.freeze
    @deadline = Float(deadline)
    @environment = self.class.frozen_environment(environment)
    freeze
  end

  def run(command, request)
    return failed("start") unless runnable?(command)
    return failed("timeout") if YonClock.time_left(deadline) <= 0

    payload = request_payload(request)
    return failed("start") if payload.nil?

    child = YonRelayChild.start(command, environment, payload)
    return failed("start") if child.nil?

    finish(child)
  end

  private

  def failed(category)
    EVENTS.write(JSON.generate({ "event" => "handler.relay_failed", "request_id" => request_id, "category" => category }) + "\n")
    [502, { "content-type" => "application/json" }, [FAILURE_BODY.dup]]
  end

  def runnable?(command)
    command.is_a?(Array) && !command.empty? && command.all?(String)
  end

  def request_payload(request)
    JSON.generate(YonHttp.message(request), strict: true)
  rescue StandardError
    nil
  end

  def finish(child)
    status = child.finish(deadline)
    response(child, status)
  ensure
    child.stop
  end

  def response(child, status)
    return failed("timeout") if status.nil?
    return failed("overflow") if child.overflowed?
    return failed("exit") unless status.is_a?(Process::Status) && status.success?

    envelope = decode(child.stdout)
    return failed("protocol") unless envelope.is_a?(Hash) && YonHttp.status?(envelope.fetch("status", 200))

    body = envelope.fetch("body", "")
    [envelope.fetch("status", 200), envelope.fetch("headers", {}), [body.is_a?(String) ? body : JSON.generate(body)]]
  end

  def decode(output)
    text = output.dup.force_encoding(Encoding::UTF_8)
    return nil unless text.valid_encoding?

    JSON.parse(text)
  rescue JSON::ParserError
    nil
  end

  freeze
end

class YonRunnerLimits
  MAX_FRAME_BYTES = 16 * 1024 * 1024
  MAX_MESSAGE_CHARACTERS = 2048
  PROTOCOL_ERROR_EXIT_CODE = 70
  RESULT_POLL_SECONDS = 0.02
  DEFAULT_DEADLINE_MS = 30_000
  MIDDLEWARE_PHASES = { "middleware.before" => "before", "middleware.after" => "after" }.freeze
  SHAPE_ERRORS = [TypeError, ArgumentError, JSON::GeneratorError, JSON::NestingError, EncodingError].freeze
  NOT_SERIALIZABLE = "Handler returned a value that is not JSON-serializable."
  FRAME_TOO_LARGE = "Serialized handler result exceeds the protocol frame limit."
  EVENT_TOO_LARGE = "Streamed event exceeds the protocol frame limit."
  JSON_CONTENT_TYPE = "application/json; charset=utf-8"
end

class YonNoRoute < StandardError
end

class YonRunnerJson
  def self.value(value)
    JSON.generate(value, strict: true, allow_nan: false)
  rescue JSON::GeneratorError, JSON::NestingError, EncodingError
    raise TypeError, YonRunnerLimits::NOT_SERIALIZABLE
  end

  def self.frame_payload(envelope)
    payload = JSON.generate(envelope, strict: true, allow_nan: false)
    return failure_payload(envelope, YonRunnerLimits::FRAME_TOO_LARGE) if payload.bytesize > YonRunnerLimits::MAX_FRAME_BYTES
    payload
  rescue StandardError
    failure_payload(envelope, YonRunnerLimits::NOT_SERIALIZABLE)
  end

  def self.failure_payload(envelope, message)
    JSON.generate(YonRunnerFailures.build(envelope["request_id"], "TY2203", message))
  end

  def self.parse(payload)
    text = payload.force_encoding(Encoding::UTF_8)
    return nil unless text.valid_encoding?
    JSON.parse(text)
  rescue JSON::ParserError
    nil
  end
end

class YonRunnerFailures
  def self.build(request_id, code, message, status = 500)
    {
      "protocol_version" => 1,
      "kind" => "response",
      "request_id" => request_id,
      "status" => status,
      "headers" => {},
      "error" => { "code" => code, "message" => public_message(message), "retryable" => false }
    }
  end

  def self.public_message(message)
    text = message.to_s.dup.force_encoding(Encoding::UTF_8).scrub
    text = text[0, YonRunnerLimits::MAX_MESSAGE_CHARACTERS]
    return "Handler failed." if text.empty?
    text
  end

  def self.for_error(request_id, error)
    return build(request_id, "TY2203", error.message) if shape_error?(error)
    build(request_id, "TY2201", error.message)
  end

  def self.shape_error?(error)
    YonRunnerLimits::SHAPE_ERRORS.any? { |kind| error.is_a?(kind) }
  end
end

class YonRunnerFrames
  @lock = Mutex.new
  @output = nil

  def self.take_standard_output
    @output = STDOUT.dup
    @output.binmode
    @output.sync = true
    STDOUT.reopen(STDERR)
    STDOUT.sync = true
  end

  def self.write(envelope)
    write_payload(YonRunnerJson.frame_payload(envelope))
  end

  def self.write_payload(payload)
    @lock.synchronize do
      @output.write([payload.bytesize].pack("N"), payload)
    end
  end

  def self.protocol_error
    exit!(YonRunnerLimits::PROTOCOL_ERROR_EXIT_CODE)
  end

  def self.read_all(incoming)
    STDIN.binmode
    loop do
      prefix = STDIN.read(4)
      break if prefix.nil? || prefix.bytesize < 4
      incoming.push(read_payload(prefix.unpack1("N")))
    end
    incoming.push(:closed)
  end

  def self.read_payload(length)
    protocol_error if length > YonRunnerLimits::MAX_FRAME_BYTES
    payload = STDIN.read(length)
    protocol_error if payload.nil? || payload.bytesize < length
    envelope = YonRunnerJson.parse(payload)
    protocol_error if envelope.nil?
    envelope
  end
end

class YonModels
  def self.read(type, value, field_types = {})
    return nil if value.nil?
    return list(type[0], value, field_types) if type.is_a?(Array)
    return value unless type.is_a?(Class) && value.is_a?(Hash)
    return type.new(**record_fields(type, value, field_types)) if record_class?(type)
    return type.new(**keyword_fields(type, value, field_types)) if keyword_class?(type)
    value
  end

  def self.list(type, value, field_types)
    return value unless value.is_a?(Array)
    value.map { |entry| read(type, entry, field_types) }
  end

  def self.record_class?(type)
    type < Data || type < Struct
  end

  def self.keyword_class?(type)
    keyword_names(type).any?
  end

  def self.keyword_names(type)
    parameters = type.instance_method(:initialize).parameters
    keywords = parameters.select { |kind, _name| kind == :key || kind == :keyreq }
    keywords.map { |_kind, name| name }
  end

  def self.record_fields(type, value, field_types)
    type.members.to_h { |name| [name, field(type, name, value, field_types)] }
  end

  def self.keyword_fields(type, value, field_types)
    names = keyword_names(type).select { |name| value.key?(name.to_s) }
    names.to_h { |name| [name, field(type, name, value, field_types)] }
  end

  def self.field(type, name, value, field_types)
    entry = value[name.to_s]
    declared = field_types.fetch(type, {})[name.to_s]
    return entry if declared.nil?
    read(declared, entry, field_types)
  end

  def self.plain(value)
    return plain(value.to_h) if value.is_a?(Data) || value.is_a?(Struct)
    return plain_hash(value) if value.is_a?(Hash)
    return value.map { |entry| plain(entry) } if value.is_a?(Array) || value.is_a?(Set)
    return value.to_s if value.is_a?(Symbol)
    value
  end

  def self.plain_hash(value)
    value.to_h { |key, entry| [plain_key(key), plain(entry)] }
  end

  def self.plain_key(key)
    return key.to_s if key.is_a?(Symbol) || key.is_a?(Integer)
    key
  end
end

class YonHttp
  ANSWER_ADVICE = "a [status, headers, body] array"

  def self.request(frame)
    headers = frame["headers"] || {}
    host = (headers["host"] || ["localhost"]).first
    uri = URI("http://#{host}#{frame["target"] || frame["route"] || "/"}")
    request = Net::HTTP.const_get(frame["method"].to_s.capitalize).new(uri)
    request.each_name.to_a.each { |name| request.delete(name) }
    headers.each { |name, values| values.each { |value| request.add_field(name, value) } }
    request.body = body(frame) if request.request_body_permitted?
    request
  end

  def self.body(frame)
    body = frame["body"]
    return nil if body.nil?
    return body["data"].to_s.unpack1("m") if body["encoding"] == "base64"

    body["data"].to_s
  end

  def self.message(request)
    body = request.body.to_s.dup.force_encoding(Encoding::UTF_8)
    body = body.scrub unless body.valid_encoding?
    { "route" => request.path, "method" => request.method, "headers" => request.to_hash, "body" => { "encoding" => "utf8", "data" => body } }
  end

  def self.status?(value)
    value.is_a?(Integer) && value.between?(100, 599)
  end

  def self.answer?(value)
    value.is_a?(Array) && value.size == 3 && status?(value[0]) && value[1].is_a?(Hash)
  end

  def self.text(body)
    return "" if body.nil?
    return body if body.is_a?(String)
    return body.map(&:to_s).join if body.is_a?(Array) && body.all?(String)

    YonRunnerJson.value(YonModels.plain(body))
  end
end

class YonRunnerResponses
  def self.for_result(request_id, result)
    return explicit(request_id, result) if YonHttp.answer?(result)
    data = YonRunnerJson.value(YonModels.plain(result))
    response = envelope(request_id, 200, { "content-type" => [YonRunnerLimits::JSON_CONTENT_TYPE] })
    response["body"] = { "encoding" => "utf8", "data" => data }
    response
  end

  def self.explicit(request_id, value)
    status, raw_headers, body = value
    response = envelope(request_id, status, headers(raw_headers))
    response["body"] = { "encoding" => "utf8", "data" => YonHttp.text(body) }
    response
  end

  def self.envelope(request_id, status, headers)
    { "protocol_version" => 1, "kind" => "response", "request_id" => request_id, "status" => status, "headers" => headers }
  end

  def self.headers(raw_headers)
    raise TypeError, "Handler response headers must be an object." unless raw_headers.is_a?(Hash)
    raw_headers.to_h { |name, raw| [name.downcase, header_values(name, raw)] }
  end

  def self.header_values(name, raw)
    raise TypeError, "Handler response header names must be strings." unless name.is_a?(String)
    values = raw.is_a?(Array) ? raw : [raw]
    return values if values.any? && values.all? { |item| item.is_a?(String) }
    raise TypeError, "Handler response header '#{name}' must contain strings."
  end

end

class YonErrors
  def self.handle(error, candidates)
    raise error unless error.is_a?(StandardError)
    candidates.each { |candidate| check_type(candidate[0]) }
    selected = nearest(error, candidates)
    raise error if selected.nil?
    answer(selected[1], selected[2].call)
  end

  def self.check_type(type)
    return if type.is_a?(Class) && type <= StandardError
    raise TypeError, "HandlesError requires an exception type"
  end

  def self.nearest(error, candidates)
    selected = nil
    selected_distance = nil
    candidates.each do |candidate|
      distance = error.class.ancestors.index(candidate[0])
      next if distance.nil?
      next if selected_distance && distance >= selected_distance
      selected = candidate
      selected_distance = distance
    end
    selected
  end

  def self.answer(status, result)
    return [result[0] == 200 ? status : result[0], result[1], result[2]] if YonHttp.answer?(result)

    [status, { "content-type" => "application/json" }, [YonRunnerJson.value(YonModels.plain(result))]]
  end
end

class YonRunnerModels
  FOLDER = "server/models"

  def self.load(root, wrap)
    pending = Dir.glob("#{FOLDER}/**/*.rb", base: root).sort
    until pending.empty?
      failed = pending.reject { |path| load_file(root, path, wrap) }
      raise_first_failure(root, failed, wrap) if failed.size == pending.size
      pending = failed
    end
  end

  def self.load_file(root, path, wrap)
    source = File.read(File.join(root, path), encoding: Encoding::UTF_8)
    wrap.module_eval(source, path, 1)
    true
  rescue NameError
    false
  end

  def self.raise_first_failure(root, failed, wrap)
    source = File.read(File.join(root, failed.first), encoding: Encoding::UTF_8)
    wrap.module_eval(source, failed.first, 1)
  end
end

class YonRunnerLoader
  def self.entry(source, root, relay_program, deadline, class_name)
    path = File.expand_path(source)
    $LOAD_PATH.unshift(File.expand_path(root))
    $LOAD_PATH.unshift(File.dirname(path))
    install_names(relay_program, deadline)
    wrap = Module.new
    YonRunnerModels.load(File.expand_path(root), wrap)
    load(path, wrap)
    entry_class(wrap, class_name)
  end

  def self.install_names(relay_program, deadline)
    Object.const_set(:YonGateway, YonGatewayClient.new(deadline).freeze)
    return if Object.method_defined?(:relay) || Object.private_method_defined?(:relay)
    Kernel.module_eval do
      define_method(:relay) { |command, request| relay_program.run(command, request) }
      private :relay
    end
  end

  def self.entry_class(wrap, class_name)
    entry = wrap.const_get(class_name, false) if wrap.const_defined?(class_name, false)
    return entry if entry.is_a?(Class)
    raise TypeError, "Module must define a class named #{class_name}."
  end
end

class YonRunnerInvocation
  def self.run(handler, request, completed)
    completed.push(answer(handler, request))
  rescue Exception => error
    completed.push(YonRunnerFailures.for_error(request["request_id"], error))
  end

  def self.answer(handler, request)
    phase = YonRunnerLimits::MIDDLEWARE_PHASES[request["operation"]]
    return middleware(handler, request, phase) unless phase.nil?
    return deliver(handler, request) if request["operation"] == "event.deliver"
    name = request["method"].to_s
    unless public_class_method?(handler, name)
      return YonRunnerFailures.build(request["request_id"], "TY2202", "Handler does not define self.#{name}().", 405)
    end
    argument = handler.const_defined?(:YON_FRAMES, false) ? request : YonHttp.request(request)
    result = call(handler, name, argument)
    stream(request["request_id"], result) if result.is_a?(Enumerator)
    YonRunnerResponses.for_result(request["request_id"], result)
  end

  def self.deliver(handler, request)
    unless public_class_method?(handler, "yon_deliver")
      return YonRunnerFailures.build(request["request_id"], "TY2202", "Handler has no topic listeners.", 404)
    end
    handler.yon_deliver(JSON.parse(request.dig("body", "data") || "{}"))
    YonRunnerResponses.for_result(request["request_id"], nil)
  end

  def self.middleware(handler, request, phase)
    passed = [204, {}, []]
    return YonRunnerResponses.for_result(request["request_id"], passed) unless phase_present?(handler, phase)
    unless public_class_method?(handler, phase)
      return YonRunnerFailures.build(request["request_id"], "TY2202", "Middleware does not define self.#{phase}().")
    end
    result = call(handler, phase, YonHttp.request(request)) || passed
    return YonRunnerResponses.for_result(request["request_id"], result) if YonHttp.answer?(result)

    message = "Middleware #{phase}() must return #{YonHttp::ANSWER_ADVICE}, or nil to let the request pass."
    YonRunnerFailures.build(request["request_id"], "TY2201", message)
  end

  def self.call(handler, name, request)
    return handler.public_send(name) if handler.method(name).arity.zero?

    handler.public_send(name, request)
  end

  def self.public_class_method?(klass, name)
    return false unless klass.singleton_class.public_method_defined?(name)
    owner = klass.singleton_class.instance_method(name).owner
    !Class.ancestors.include?(owner)
  end

  def self.phase_present?(klass, phase)
    return true if klass.singleton_class.method_defined?(phase) || klass.singleton_class.private_method_defined?(phase)
    klass.method_defined?(phase) || klass.private_method_defined?(phase)
  end

  def self.stream(request_id, events)
    begin
      events.each { |event| write_event(request_id, event) }
    rescue Exception
      YonRunnerFrames.write(YonRunnerFailures.build(request_id, "TY2204", "The handler stream failed."))
    end
    exit!(0)
  end

  def self.write_event(request_id, event)
    data = YonRunnerJson.value(YonModels.plain(event))
    body = { "encoding" => "utf8", "data" => data }
    envelope = { "protocol_version" => 1, "kind" => "event", "request_id" => request_id, "body" => body }
    payload = JSON.generate(envelope, strict: true)
    oversized_event(request_id) if payload.bytesize > YonRunnerLimits::MAX_FRAME_BYTES
    YonRunnerFrames.write_payload(payload)
  end

  def self.oversized_event(request_id)
    YonRunnerFrames.write(YonRunnerFailures.build(request_id, "TY2203", YonRunnerLimits::EVENT_TOO_LARGE))
    exit!(0)
  end
end

class SocketPeer
  attr_reader :id

  def initialize(session, id)
    @session = session
    @id = id
  end

  def send(message)
    @session.send_to(message, @id)
  end
end

class SocketSession
  attr_reader :id

  def initialize(id, incoming)
    @id = id
    @incoming = incoming
    @inbox = []
    @closed = false
    @sessions = nil
  end

  def send(message)
    send_to(message, nil)
  end

  def close
    @closed = true
    write({ "kind" => "close" })
  end

  def open_sessions
    @sessions = nil
    write({ "kind" => "sessions" })
    receive while @sessions.nil? && !@closed
    (@sessions || []).map { |peer| SocketPeer.new(self, peer.to_s) }
  end

  def send_to(message, to)
    data = YonRunnerJson.value(YonModels.plain(message))
    frame = { "kind" => "event", "body" => { "encoding" => "utf8", "data" => data } }
    frame["to"] = to unless to.nil?
    write(frame)
  end

  def next_message
    receive while @inbox.empty? && !@closed
    return YonSocket::ENDED if @inbox.empty?

    @inbox.shift
  end

  private

  def receive
    envelope = @incoming.pop
    unless envelope.is_a?(Hash)
      @closed = true
      return
    end
    YonRunnerFrames.protocol_error unless envelope["request_id"] == @id
    case envelope["kind"]
    when "message" then @inbox.push(JSON.parse(envelope.dig("body", "data").to_s))
    when "sessions" then @sessions = envelope["sessions"].is_a?(Array) ? envelope["sessions"] : []
    else @closed = true
    end
  end

  def write(frame)
    YonRunnerFrames.write({ "protocol_version" => 1, "request_id" => @id }.merge(frame))
  end
end

class YonSocket
  ENDED = Object.new.freeze
  CALL_SECONDS = 30
  @incoming = nil

  class << self
    attr_accessor :incoming
  end

  def self.run(endpoint, request)
    session = SocketSession.new(request["request_id"], incoming)
    attempt(endpoint, session) { endpoint.open(session) }
    loop do
      message = session.next_message
      break if message.equal?(ENDED)

      attempt(endpoint, session) { endpoint.message(session, message) }
    end
    begin
      call { endpoint.close(session) }
    rescue StandardError
      nil
    end
    exit!(0)
  end

  def self.call
    Object.send(:remove_const, :YonGateway) if Object.const_defined?(:YonGateway, false)
    Object.const_set(:YonGateway, YonGatewayClient.new(YonClock.now + CALL_SECONDS).freeze)
    yield
  end

  def self.attempt(endpoint, session, &action)
    call(&action)
  rescue StandardError => failure
    line = { "event" => "handler.socket_failed", "request_id" => session.id, "message" => failure.message }
    STDERR.puts(JSON.generate(line))
    begin
      call { endpoint.error(session, failure) }
    rescue StandardError
      nil
    end
  end
end

class YonRunner
  @incoming = Thread::Queue.new
  @completed = Thread::Queue.new

  def self.main(arguments)
    started_at = YonClock.now
    environment = YonRelayProgram.environment(ENV.to_h)
    YonRunnerFrames.take_standard_output
    Thread.new { YonRunnerFrames.read_all(@incoming) }
    request = @incoming.pop
    YonRunnerFrames.protocol_error unless request?(request)
    deadline = started_at + deadline_ms(request) / 1000.0
    relay_program = YonRelayProgram.new(request_id: request["request_id"], deadline: deadline, environment: environment)
    handler = load_handler(arguments, relay_program, deadline, request)
    if request["operation"] == "socket.open"
      YonSocket.incoming = @incoming
      YonRunnerInvocation.run(handler, request, @completed)
      finish(@completed.pop)
    end
    Thread.new { YonRunnerInvocation.run(handler, request, @completed) }
    wait_for_result(request["request_id"])
  end

  def self.request?(envelope)
    return false unless envelope.is_a?(Hash)
    return false unless envelope["protocol_version"] == 1 && envelope["kind"] == "request"
    envelope["request_id"].is_a?(String)
  end

  def self.deadline_ms(request)
    milliseconds = request["deadline_ms"]
    return YonRunnerLimits::DEFAULT_DEADLINE_MS unless milliseconds.is_a?(Numeric)
    milliseconds
  end

  def self.load_handler(arguments, relay_program, deadline, request)
    class_name = "Handler"
    class_name = "Middleware" if YonRunnerLimits::MIDDLEWARE_PHASES.key?(request["operation"])
    YonRunnerLoader.entry(arguments[0], arguments[1], relay_program, deadline, class_name)
  rescue Exception => error
    finish(YonRunnerFailures.build(request["request_id"], "TY2201", error.message))
  end

  def self.wait_for_result(request_id)
    loop do
      response = @completed.pop(timeout: YonRunnerLimits::RESULT_POLL_SECONDS)
      finish(response) unless response.nil?
      next unless cancel_requested?(request_id)
      finish(YonRunnerFailures.build(request_id, "TY2111", "Handler invocation was cancelled.", 499))
    end
  end

  def self.cancel_requested?(request_id)
    return false if @incoming.empty?
    envelope = @incoming.pop
    envelope.is_a?(Hash) && envelope["kind"] == "cancel" && envelope["request_id"] == request_id
  end

  def self.finish(response)
    YonRunnerFrames.write(response)
    exit!(0)
  end
end

YonRunner.main(ARGV)
