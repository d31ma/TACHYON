import asyncio
import base64
import builtins
import collections.abc
import contextlib
import dataclasses
import enum
import http
import http.client
import importlib.util
import inspect
import json
import os
import queue
import re
import struct
import subprocess
import sys
import threading
import time
import types
import typing
import urllib.parse
import urllib.request


class ProtocolLimits:
    MAX_FRAME_BYTES = 16 * 1024 * 1024
    MAX_RELAY_STDOUT_BYTES = 16 * 1024 * 1024
    MAX_RELAY_STDERR_BYTES = 64 * 1024
    MAX_RESOURCE_BYTES = 1024 * 1024
    MAX_MESSAGE_CHARACTERS = 2048


class RunnerSettings:
    RESOURCE_TIMEOUT_SECONDS = 5
    SCROLL_TIMEOUT_SECONDS = 30
    GATEWAY_ANSWER_MARGIN_SECONDS = 1
    RELAY_READ_BYTES = 8192
    DEFAULT_RELAY_DEADLINE_MS = 30_000
    RELAY_FAILURE_MESSAGE = "Component invocation failed."
    RELAY_POLL_SECONDS = 0.002
    RESULT_POLL_SECONDS = 0.02
    PROTOCOL_ERROR_EXIT_CODE = 70


class MiddlewarePhases:
    BY_OPERATION = {"middleware.before": "before", "middleware.after": "after"}


class RunnerState:
    started_at = time.monotonic()
    protocol_output = sys.stdout.buffer
    source = sys.argv[1]
    project_root = sys.argv[2]
    incoming = queue.Queue()
    completed = queue.Queue()
    relay_program = None


def invalid_resource_number(_value):
    raise ValueError("Invalid JSON number")


def resource_settings():
    endpoint = re.fullmatch(r"http://127\.0\.0\.1:([0-9]+)/", os.environ.get("YON_RESOURCE_ENDPOINT", ""))
    token = os.environ.get("YON_RESOURCE_TOKEN", "")
    if endpoint is None or re.fullmatch(r"[a-f0-9]{64}", token) is None:
        raise ValueError()
    return int(endpoint.group(1)), token


def encode_resource_command(command):
    body = json.dumps(command, allow_nan=False, separators=(",", ":")).encode("utf-8")
    if len(body) > ProtocolLimits.MAX_RESOURCE_BYTES:
        raise ValueError()
    return body


def post_resource(body, timeout_seconds):
    connection = None
    try:
        port, token = resource_settings()
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=timeout_seconds)
        headers = {"Content-Type": "application/json", "Authorization": "Bearer " + token}
        connection.request("POST", "/", body=body, headers=headers)
        response = connection.getresponse()
        result = response.read(ProtocolLimits.MAX_RESOURCE_BYTES + 1)
        if response.status != 200 or len(result) > ProtocolLimits.MAX_RESOURCE_BYTES:
            raise ValueError()
        return json.loads(result.decode("utf-8"), parse_constant=invalid_resource_number)
    finally:
        if connection is not None:
            connection.close()


def managed_resource(command, timeout_seconds=RunnerSettings.RESOURCE_TIMEOUT_SECONDS):
    try:
        body = encode_resource_command(command)
        return post_resource(body, timeout_seconds)
    except Exception:
        raise RuntimeError("Yon managed resource operation failed.") from None


class TopicClient:
    @staticmethod
    def publish(topic, payload):
        return managed_resource({"operation": "topic", "topic": topic, "payload": payload})


def gateway_answer(answer):
    if not isinstance(answer, dict):
        return False
    return "result" in answer or isinstance(answer.get("error"), str)


@dataclasses.dataclass(frozen=True)
class GatewayClient:
    deadline: float

    def call(self, key, arguments):
        try:
            body = encode_resource_command({"operation": "gateway", "key": key, "arguments": list(arguments)})
        except (TypeError, ValueError):
            return {"error": "invalid"}
        if time_left(self.deadline) <= 0:
            return {"error": "timeout"}
        try:
            answer = post_resource(body, time_left(self.deadline) + RunnerSettings.GATEWAY_ANSWER_MARGIN_SECONDS)
        except Exception:
            if time_left(self.deadline) <= 0:
                return {"error": "timeout"}
            return {"error": "adapter"}
        if gateway_answer(answer):
            return answer
        return {"error": "adapter"}


class DataSource:
    def command(self, command):
        return managed_resource(command)

    def query(self, sql, parameters=()):
        return managed_resource({"operation": "sql", "sql": sql, "parameters": list(parameters), "mode": "rows"})

    def execute(self, sql, parameters=()):
        if not isinstance(sql, str):
            return managed_resource(sql)
        return managed_resource({"operation": "sql", "sql": sql, "parameters": list(parameters), "mode": "execute"})

    def find_by_id(self, collection, id):
        return document_command("findById", {"collection": collection, "id": id})

    def insert(self, collection, document):
        return document_command("insert", {"collection": collection, "value": document})

    def patch(self, collection, id, fields):
        return document_command("patch", {"collection": collection, "id": id, "value": fields})

    def delete_by_id(self, collection, id):
        return document_command("deleteById", {"collection": collection, "id": id})

    def scroll(self, collection, where=None, cursor=None, size=20):
        fields = {"collection": collection, "where": where, "cursor": cursor, "size": size}
        return document_command("scroll", fields, RunnerSettings.SCROLL_TIMEOUT_SECONDS)

    def create_collection(self, name):
        return document_command("createCollection", {"collection": name})

    def drop_collection(self, name):
        return document_command("dropCollection", {"collection": name})

    def collection_exists(self, name):
        return document_command("collectionExists", {"collection": name})

    def publish(self, topic, payload, key=None):
        return queue_command("publish", {"topic": topic, "payload": payload, "key": key})

    def claim(self, topic, group, max_messages=1, lease_ms=30000):
        return queue_command("claim", {"topic": topic, "group": group, "maxMessages": max_messages, "leaseMs": lease_ms})

    def ack(self, topic, group, delivery):
        return queue_command("ack", {"topic": topic, "group": group, "id": delivery["id"], "receipt": delivery["receipt"]})

    def nack(self, topic, group, delivery, delay_ms=0):
        fields = {"topic": topic, "group": group, "id": delivery["id"], "receipt": delivery["receipt"], "delayMs": delay_ms}
        return queue_command("nack", fields)


def document_command(command, fields, timeout_seconds=RunnerSettings.RESOURCE_TIMEOUT_SECONDS):
    return managed_resource({"operation": "document", "command": command, **fields}, timeout_seconds)


def queue_command(command, fields):
    return managed_resource({"operation": "queue", "command": command, **fields})


class RedisTemplate:
    def get(self, key):
        return managed_resource({"operation": "redis", "command": "get", "key": key})

    def set(self, key, value, ttl_seconds):
        return managed_resource({"operation": "redis", "command": "set", "key": key, "value": value, "ttlSeconds": ttl_seconds})

    def delete(self, key):
        return managed_resource({"operation": "redis", "command": "delete", "key": key})


def public_message(error):
    value = str(error) or "Handler failed."
    return value[:ProtocolLimits.MAX_MESSAGE_CHARACTERS]


def serialization_failure(envelope, message):
    response = {
        "protocol_version": 1,
        "kind": "response",
        "request_id": envelope.get("request_id", "adapter_error"),
        "status": 500,
        "headers": {},
        "error": {"code": "TY2203", "message": message, "retryable": False},
    }
    return json.dumps(response, separators=(",", ":")).encode("utf-8")


def write_frame(envelope):
    try:
        payload = json.dumps(envelope, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    except Exception:
        payload = serialization_failure(envelope, "Handler returned a value that is not JSON-serializable.")
    if len(payload) > ProtocolLimits.MAX_FRAME_BYTES:
        payload = serialization_failure(envelope, "Serialized handler result exceeds the protocol frame limit.")
    RunnerState.protocol_output.write(struct.pack(">I", len(payload)))
    RunnerState.protocol_output.write(payload)
    RunnerState.protocol_output.flush()


def failure(request_id, code, message, status=500):
    return {
        "protocol_version": 1,
        "kind": "response",
        "request_id": request_id,
        "status": status,
        "headers": {},
        "error": {
            "code": code,
            "message": str(message)[:ProtocolLimits.MAX_MESSAGE_CHARACTERS] or "Handler failed.",
            "retryable": False,
        },
    }


def read_exact(stream, length):
    parts = bytearray()
    while len(parts) < length:
        part = stream.read(length - len(parts))
        if not part:
            return None
        parts.extend(part)
    return bytes(parts)


def exit_with_protocol_error():
    os._exit(RunnerSettings.PROTOCOL_ERROR_EXIT_CODE)


def read_frames():
    while True:
        prefix = read_exact(sys.stdin.buffer, 4)
        if prefix is None:
            return
        length = struct.unpack(">I", prefix)[0]
        if length > ProtocolLimits.MAX_FRAME_BYTES:
            exit_with_protocol_error()
        payload = read_exact(sys.stdin.buffer, length)
        if payload is None:
            exit_with_protocol_error()
        try:
            RunnerState.incoming.put(json.loads(payload.decode("utf-8")))
        except Exception:
            exit_with_protocol_error()
    RunnerState.incoming.put(None)


def collect_output(stream, limit, result):
    captured = bytearray()
    oversized = False
    try:
        while True:
            part = stream.read(RunnerSettings.RELAY_READ_BYTES)
            if not part:
                break
            remaining = limit - len(captured)
            if remaining > 0:
                captured.extend(part[:remaining])
            if len(part) > remaining:
                oversized = True
    finally:
        stream.close()
        result.extend((bytes(captured), oversized))


def start_output_thread(stream, limit, result):
    thread = threading.Thread(target=collect_output, args=(stream, limit, result), daemon=True)
    thread.start()
    return thread


def relay_environment(environment):
    kept = {}
    for name, value in environment.items():
        upper = name.upper()
        if upper == "YON_APPLICATION_JSON" or upper.startswith("YON_RESOURCE_"):
            continue
        kept[name] = value
    return kept


def invocation_deadline(request):
    milliseconds = request.get("deadline_ms", RunnerSettings.DEFAULT_RELAY_DEADLINE_MS)
    if isinstance(milliseconds, bool) or not isinstance(milliseconds, (int, float)):
        milliseconds = RunnerSettings.DEFAULT_RELAY_DEADLINE_MS
    return RunnerState.started_at + float(milliseconds) / 1000.0


def time_left(deadline):
    return deadline - time.monotonic()


def send_relay_request(child, payload):
    try:
        child.stdin.write(payload)
    except (BrokenPipeError, OSError, ValueError):
        pass
    finally:
        with contextlib.suppress(OSError):
            child.stdin.close()


def start_input_thread(child, request):
    payload = json.dumps(YonHttp.message(request)).encode("utf-8")
    thread = threading.Thread(target=send_relay_request, args=(child, payload), daemon=True)
    thread.start()


def start_relay_child(command, environment):
    try:
        return subprocess.Popen(
            list(command),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
        )
    except OSError:
        return None


def wait_for_relay_exit(child, deadline):
    while child.poll() is None and time_left(deadline) > 0:
        time.sleep(RunnerSettings.RELAY_POLL_SECONDS)
    timed_out = child.poll() is None
    if timed_out:
        child.kill()
    return timed_out


class YonHttp:
    ANSWER_ADVICE = "a (HTTPStatus, headers, body) tuple"

    @staticmethod
    def request(frame):
        headers = frame.get("headers") or {}
        host = (headers.get("host") or ["localhost"])[0]
        url = "http://" + host + (frame.get("target") or frame.get("route") or "/")
        joined = {name: ", ".join(values) for name, values in headers.items()}
        return urllib.request.Request(url, data=YonHttp.bytes(frame), headers=joined, method=frame.get("method"))

    @staticmethod
    def bytes(frame):
        body = frame.get("body")
        if not body or frame.get("method") in ("GET", "HEAD"):
            return None
        if body.get("encoding") == "base64":
            return base64.b64decode(body.get("data", ""))
        return body.get("data", "").encode("utf-8")

    @staticmethod
    def message(request):
        address = urllib.parse.urlsplit(request.full_url)
        route = address.path or "/"
        if address.query:
            route += "?" + address.query
        headers = {name.lower(): [value] for name, value in request.header_items()}
        data = request.data or b""
        if isinstance(data, str):
            data = data.encode("utf-8")
        return {
            "route": route,
            "method": request.get_method(),
            "headers": headers,
            "body": {"encoding": "utf8", "data": data.decode("utf-8", "replace")},
        }

    @staticmethod
    def is_status(value):
        return isinstance(value, int) and not isinstance(value, bool) and 100 <= value <= 599

    @staticmethod
    def is_answer(value):
        return (
            isinstance(value, tuple)
            and len(value) == 3
            and YonHttp.is_status(value[0])
            and isinstance(value[1], dict)
        )

    @staticmethod
    def answer(value):
        status, raw_headers, body = value
        return {"status": int(status), "headers": response_headers(raw_headers), "body": response_body(body)}


@dataclasses.dataclass(frozen=True)
class RelayProgram:
    request_id: str
    deadline: float
    environment: dict

    def failed(self, category):
        sys.stderr.write(json.dumps({
            "event": "handler.relay_failed",
            "request_id": self.request_id,
            "category": category,
        }, separators=(",", ":")) + "\n")
        body = json.dumps({"error": RunnerSettings.RELAY_FAILURE_MESSAGE})
        return (http.HTTPStatus.BAD_GATEWAY, {"content-type": "application/json"}, body)

    def run(self, command, request):
        if not command:
            return self.failed("start")
        if time_left(self.deadline) <= 0:
            return self.failed("timeout")
        child = start_relay_child(command, self.environment)
        if child is None:
            return self.failed("start")
        stdout_result = []
        stderr_result = []
        stdout_thread = start_output_thread(child.stdout, ProtocolLimits.MAX_RELAY_STDOUT_BYTES, stdout_result)
        stderr_thread = start_output_thread(child.stderr, ProtocolLimits.MAX_RELAY_STDERR_BYTES, stderr_result)
        start_input_thread(child, request)
        timed_out = wait_for_relay_exit(child, self.deadline)
        stdout_thread.join(max(0.0, time_left(self.deadline)))
        stderr_thread.join(max(0.0, time_left(self.deadline)))
        if stdout_thread.is_alive() or stderr_thread.is_alive():
            child.kill()
            return self.failed("timeout")
        child.wait()
        return self.response(child, timed_out, stdout_result, stderr_result)

    def response(self, child, timed_out, stdout_result, stderr_result):
        stdout, stdout_oversized = stdout_result
        _, stderr_oversized = stderr_result
        if timed_out:
            return self.failed("timeout")
        if stdout_oversized or stderr_oversized:
            return self.failed("overflow")
        if child.returncode != 0:
            return self.failed("exit")
        try:
            envelope = json.loads(stdout.decode("utf-8"))
        except (UnicodeDecodeError, ValueError):
            return self.failed("protocol")
        if not isinstance(envelope, dict) or not YonHttp.is_status(envelope.get("status", 200)):
            return self.failed("protocol")
        status = http.HTTPStatus(envelope.get("status", 200))
        return (status, envelope.get("headers", {}), envelope.get("body", ""))


class YonModels:
    LIST_TYPES = (list, tuple, set, collections.abc.Sequence, collections.abc.Iterable)

    @staticmethod
    def read(kind, value):
        if value is None:
            return None
        origin = typing.get_origin(kind)
        arguments = typing.get_args(kind)
        if origin in YonModels.LIST_TYPES:
            item = arguments[0] if arguments else typing.Any
            return [YonModels.read(item, entry) for entry in value]
        if origin is typing.Union or origin is types.UnionType:
            kept = [argument for argument in arguments if argument is not type(None)]
            if len(kept) == 1:
                return YonModels.read(kept[0], value)
            return value
        if isinstance(kind, type) and issubclass(kind, enum.Enum):
            return kind(value)
        if YonModels.is_record(kind) and isinstance(value, dict):
            return YonModels.record(kind, value)
        return value

    @staticmethod
    def is_record(kind):
        if dataclasses.is_dataclass(kind):
            return True
        return isinstance(kind, type) and issubclass(kind, tuple) and hasattr(kind, "_fields")

    @staticmethod
    def record(kind, value):
        fields = {}
        for name, hint in typing.get_type_hints(kind).items():
            if name in value:
                fields[name] = YonModels.read(hint, value[name])
        return kind(**fields)

    @staticmethod
    def plain(value):
        if isinstance(value, enum.Enum):
            return value.value
        if dataclasses.is_dataclass(value) and not isinstance(value, type):
            fields = dataclasses.fields(value)
            return {field.name: YonModels.plain(getattr(value, field.name)) for field in fields}
        if isinstance(value, tuple) and hasattr(value, "_asdict"):
            return YonModels.plain(value._asdict())
        if isinstance(value, dict):
            return {key: YonModels.plain(entry) for key, entry in value.items()}
        if isinstance(value, (list, tuple)):
            return [YonModels.plain(entry) for entry in value]
        return value


class SocketPeer:
    def __init__(self, connection, peer_id):
        self.id = peer_id
        self.connection = connection

    def send(self, message):
        self.connection.send(message, self.id)


class SocketSession:
    def __init__(self, connection):
        self.id = connection.request_id
        self.connection = connection

    def send(self, message):
        self.connection.send(message)

    def close(self):
        self.connection.close()

    def open_sessions(self):
        return self.connection.open_sessions()


class SocketConnection:
    def __init__(self, request_id):
        self.request_id = request_id
        self.inbox = collections.deque()
        self.closed = False
        self.sessions = None

    def write(self, frame):
        frame = {"protocol_version": 1, "request_id": self.request_id, **frame}
        payload = json.dumps(frame, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        if len(payload) > ProtocolLimits.MAX_FRAME_BYTES:
            raise ValueError("The socket message is too large.")
        RunnerState.protocol_output.write(struct.pack(">I", len(payload)) + payload)
        RunnerState.protocol_output.flush()

    def send(self, message, to=None):
        data = json.dumps(YonModels.plain(message), ensure_ascii=False, separators=(",", ":"), allow_nan=False)
        frame = {"kind": "event", "body": {"encoding": "utf8", "data": data}}
        if to is not None:
            frame["to"] = to
        self.write(frame)

    def close(self):
        self.closed = True
        self.write({"kind": "close"})

    def open_sessions(self):
        self.sessions = None
        self.write({"kind": "sessions"})
        while self.sessions is None and not self.closed:
            self.receive(RunnerState.incoming.get())
        return [SocketPeer(self, str(peer_id)) for peer_id in self.sessions or []]

    def receive(self, envelope):
        if envelope is None:
            self.closed = True
            return
        if not isinstance(envelope, dict) or envelope.get("request_id") != self.request_id:
            exit_with_protocol_error()
        kind = envelope.get("kind")
        if kind == "message":
            self.inbox.append(json.loads(envelope["body"]["data"]))
        elif kind == "sessions":
            sessions = envelope.get("sessions")
            self.sessions = sessions if isinstance(sessions, list) else []
        elif kind in ("close", "cancel"):
            self.closed = True
        else:
            exit_with_protocol_error()

    def next_message(self):
        while not self.inbox and not self.closed:
            self.receive(RunnerState.incoming.get())
        if self.inbox:
            return self.inbox.popleft()
        return YonSocket.ENDED


class YonSocket:
    ENDED = object()
    CALL_SECONDS = 30

    @staticmethod
    def call(loop, action, *values):
        deadline = time.monotonic() + YonSocket.CALL_SECONDS
        builtins.YonGateway = GatewayClient(deadline)
        builtins.relay = dataclasses.replace(RunnerState.relay_program, deadline=deadline).run
        result = action(*values)
        if inspect.isawaitable(result):
            loop.run_until_complete(result)

    @staticmethod
    def attempt(loop, endpoint, session, action, *values):
        try:
            YonSocket.call(loop, action, *values)
        except Exception as error:
            sys.stderr.write(json.dumps({
                "event": "handler.socket_failed",
                "request_id": session.id,
                "message": public_message(error),
            }, separators=(",", ":")) + "\n")
            try:
                YonSocket.call(loop, endpoint.error, session, error)
            except Exception:
                pass

    @staticmethod
    def run(endpoint, request):
        connection = SocketConnection(request["request_id"])
        session = SocketSession(connection)
        loop = asyncio.new_event_loop()
        YonSocket.attempt(loop, endpoint, session, endpoint.open, session)
        while True:
            message = connection.next_message()
            if message is YonSocket.ENDED:
                break
            YonSocket.attempt(loop, endpoint, session, endpoint.message, session, message)
        try:
            YonSocket.call(loop, endpoint.close, session)
        except Exception:
            pass
        loop.close()
        return YonSocket.ENDED


def install_builtins(relay_program):
    builtins.YonModels = YonModels
    builtins.YonHttp = YonHttp
    builtins.YonSocket = YonSocket
    builtins.DataSource = DataSource
    builtins.RedisTemplate = RedisTemplate
    builtins.ApplicationEventPublisher = type("ApplicationEventPublisher", (), {})
    builtins.YonTopic = TopicClient
    builtins.YonGateway = GatewayClient(relay_program.deadline)
    if not hasattr(builtins, "relay"):
        builtins.relay = relay_program.run


def middleware_phase(request):
    return MiddlewarePhases.BY_OPERATION.get(request.get("operation"))


def entry_class_name(request):
    if middleware_phase(request) is None:
        return "Handler"
    return "Middleware"


def load_handler(relay_program, class_name):
    sys.path.insert(0, os.path.dirname(RunnerState.source))
    sys.path.insert(1, RunnerState.project_root)
    specification = importlib.util.spec_from_file_location(
        "tachyon_yon_handler", RunnerState.source
    )
    if specification is None or specification.loader is None:
        raise RuntimeError("Cannot load the Python handler module.")
    install_builtins(relay_program)
    module = importlib.util.module_from_spec(specification)
    with contextlib.redirect_stdout(sys.stderr):
        specification.loader.exec_module(module)
    handler = getattr(module, class_name, None)
    if not inspect.isclass(handler):
        raise TypeError(f"Module must define a class named {class_name}.")
    return handler


def response_headers(raw_headers):
    if not isinstance(raw_headers, dict):
        raise TypeError("Handler response headers must be an object.")
    headers = {}
    for name, raw in raw_headers.items():
        if not isinstance(name, str):
            raise TypeError("Handler response header names must be strings.")
        if isinstance(raw, list):
            values = raw
        else:
            values = [raw]
        if not values or any(not isinstance(item, str) for item in values):
            raise TypeError(f"Handler response header '{name}' must contain strings.")
        headers[name.lower()] = values
    return headers


def response_body(body):
    if body is None:
        data = ""
    elif isinstance(body, bytes):
        data = body.decode("utf-8")
    elif isinstance(body, str):
        data = body
    else:
        data = json.dumps(YonModels.plain(body), ensure_ascii=False, separators=(",", ":"))
    return {"encoding": "utf8", "data": data}


def write_event(request_id, value):
    payload = json.dumps(
        {
            "protocol_version": 1,
            "kind": "event",
            "request_id": request_id,
            "body": {
                "encoding": "utf8",
                "data": json.dumps(YonModels.plain(value), ensure_ascii=False, separators=(",", ":")),
            },
        }
    ).encode("utf-8")
    if len(payload) > ProtocolLimits.MAX_FRAME_BYTES:
        write_frame(
            failure(
                request_id,
                "TY2203",
                "Streamed event exceeds the protocol frame limit.",
            )
        )
        return False
    RunnerState.protocol_output.write(struct.pack(">I", len(payload)) + payload)
    RunnerState.protocol_output.flush()
    return True


def stream_events(request_id, iterator):
    try:
        for event in iterator:
            if not write_event(request_id, event):
                os._exit(0)
    except BaseException:
        write_frame(failure(request_id, "TY2204", "The handler stream failed."))
    RunnerState.protocol_output.flush()
    os._exit(0)


def result_response(request, result):
    if YonHttp.is_answer(result):
        answer = YonHttp.answer(result)
        return {
            "protocol_version": 1,
            "kind": "response",
            "request_id": request["request_id"],
            **answer,
        }
    data = json.dumps(YonModels.plain(result), ensure_ascii=False, separators=(",", ":"), allow_nan=False)
    return {
        "protocol_version": 1,
        "kind": "response",
        "request_id": request["request_id"],
        "status": 200,
        "headers": {"content-type": ["application/json; charset=utf-8"]},
        "body": {"encoding": "utf8", "data": data},
    }


def call_handler_method(method, request):
    with contextlib.redirect_stdout(sys.stderr):
        takes_request = bool(inspect.signature(method).parameters)
        result = method(request) if takes_request else method()
        if inspect.isawaitable(result):
            result = asyncio.run(result)
    return result


def invoke_middleware(middleware, request, phase):
    descriptor = inspect.getattr_static(middleware, phase, None)
    if descriptor is None:
        RunnerState.completed.put(result_response(request, (http.HTTPStatus.NO_CONTENT, {}, "")))
        return
    if not isinstance(descriptor, staticmethod):
        message = f"Middleware does not define static {phase}()."
        RunnerState.completed.put(failure(request["request_id"], "TY2202", message))
        return
    result = call_handler_method(descriptor.__func__, YonHttp.request(request))
    if result is None:
        result = (http.HTTPStatus.NO_CONTENT, {}, "")
    if not YonHttp.is_answer(result):
        message = f"Middleware {phase}() must return {YonHttp.ANSWER_ADVICE}, or None to let the request pass."
        RunnerState.completed.put(failure(request["request_id"], "TY2201", message))
        return
    RunnerState.completed.put(result_response(request, result))


def invoke(handler, request):
    try:
        phase = middleware_phase(request)
        if phase is not None:
            invoke_middleware(handler, request, phase)
            return
        if request.get("operation") == "event.deliver":
            deliver = inspect.getattr_static(handler, "_yon_deliver", None)
            if not isinstance(deliver, staticmethod):
                RunnerState.completed.put(failure(request["request_id"], "TY2202", "Handler has no topic listeners.", 404))
                return
            call_handler_method(deliver.__func__, json.loads((request.get("body") or {}).get("data") or "{}"))
            RunnerState.completed.put(result_response(request, None))
            return
        descriptor = inspect.getattr_static(handler, request["method"], None)
        if not isinstance(descriptor, staticmethod):
            message = f"Handler does not define static {request['method']}()."
            RunnerState.completed.put(failure(request["request_id"], "TY2202", message, 405))
            return
        argument = request if getattr(handler, "__yon_frames__", False) else YonHttp.request(request)
        result = call_handler_method(descriptor.__func__, argument)
        if result is YonSocket.ENDED:
            RunnerState.protocol_output.flush()
            os._exit(0)
        if inspect.isgenerator(result):
            stream_events(request["request_id"], result)
            return
        RunnerState.completed.put(result_response(request, result))
    except (TypeError, ValueError) as error:
        RunnerState.completed.put(failure(request["request_id"], "TY2203", public_message(error)))
    except BaseException as error:
        RunnerState.completed.put(failure(request["request_id"], "TY2201", public_message(error)))


def is_request(envelope):
    return (
        isinstance(envelope, dict)
        and envelope.get("protocol_version") == 1
        and envelope.get("kind") == "request"
        and isinstance(envelope.get("request_id"), str)
    )


def is_cancel(envelope, request_id):
    return (
        isinstance(envelope, dict)
        and envelope.get("kind") == "cancel"
        and envelope.get("request_id") == request_id
    )


def finish(response):
    write_frame(response)
    os._exit(0)


def next_completed_response():
    try:
        return RunnerState.completed.get(timeout=RunnerSettings.RESULT_POLL_SECONDS)
    except queue.Empty:
        return None


def cancel_requested(request_id):
    try:
        envelope = RunnerState.incoming.get_nowait()
    except queue.Empty:
        return False
    return is_cancel(envelope, request_id)


def wait_for_result(request_id):
    while True:
        response = next_completed_response()
        if response is not None:
            finish(response)
        if cancel_requested(request_id):
            finish(failure(request_id, "TY2111", "Handler invocation was cancelled.", 499))


def main():
    environment = relay_environment(os.environ)
    threading.Thread(target=read_frames, daemon=True).start()
    request = RunnerState.incoming.get()
    if not is_request(request):
        exit_with_protocol_error()
    relay_program = RelayProgram(request["request_id"], invocation_deadline(request), environment)
    RunnerState.relay_program = relay_program
    try:
        handler = load_handler(relay_program, entry_class_name(request))
    except BaseException as error:
        finish(failure(request["request_id"], "TY2201", public_message(error)))
    if request.get("operation") == "socket.open":
        invoke(handler, request)
        finish(RunnerState.completed.get())
    threading.Thread(target=invoke, args=(handler, request), daemon=True).start()
    wait_for_result(request["request_id"])


if __name__ == "__main__":
    main()
