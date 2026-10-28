import 'dart:async' as yon_async;
import 'dart:convert' as yon_convert;
import 'dart:io' as yon_io;
import 'package:shelf/shelf.dart' as yon_shelf;
//__YON_BODY__

final class DataSource {
  yon_async.Future<Object?> query(String sql, List<Object?> parameters) =>
      YonManagedResources.call({'operation': 'sql', 'sql': sql, 'parameters': parameters, 'mode': 'rows'});
  yon_async.Future<Object?> execute(String sql, List<Object?> parameters) =>
      YonManagedResources.call({'operation': 'sql', 'sql': sql, 'parameters': parameters, 'mode': 'execute'});
  yon_async.Future<Object?> command(Map<String, Object?> command) => YonManagedResources.call(command);
  yon_async.Future<Object?> findById(String collection, String id) =>
      _document('findById', {'collection': collection, 'id': id});
  yon_async.Future<Object?> insert(String collection, Map<String, Object?> document) =>
      _document('insert', {'collection': collection, 'value': document});
  yon_async.Future<Object?> patch(String collection, String id, Map<String, Object?> fields) =>
      _document('patch', {'collection': collection, 'id': id, 'value': fields});
  yon_async.Future<Object?> deleteById(String collection, String id) =>
      _document('deleteById', {'collection': collection, 'id': id});
  yon_async.Future<Object?> scroll(String collection,
          [Map<String, Object?>? where, String? cursor, int size = 20]) =>
      YonManagedResources.call(
          {'operation': 'document', 'command': 'scroll', 'collection': collection, 'where': where, 'cursor': cursor, 'size': size},
          YonManagedResources.scrollTimeout);
  yon_async.Future<Object?> createCollection(String name) => _document('createCollection', {'collection': name});
  yon_async.Future<Object?> dropCollection(String name) => _document('dropCollection', {'collection': name});
  yon_async.Future<Object?> collectionExists(String name) => _document('collectionExists', {'collection': name});
  yon_async.Future<Object?> publish(String topic, Object? payload, [String? key]) =>
      _queue('publish', {'topic': topic, 'payload': payload, 'key': key});
  yon_async.Future<Object?> claim(String topic, String group, [int maxMessages = 1, int leaseMs = 30000]) =>
      _queue('claim', {'topic': topic, 'group': group, 'maxMessages': maxMessages, 'leaseMs': leaseMs});
  yon_async.Future<Object?> ack(String topic, String group, Map<String, Object?> delivery) =>
      _queue('ack', {'topic': topic, 'group': group, 'id': delivery['id'], 'receipt': delivery['receipt']});
  yon_async.Future<Object?> nack(String topic, String group, Map<String, Object?> delivery, [int delayMs = 0]) =>
      _queue('nack', {'topic': topic, 'group': group, 'id': delivery['id'], 'receipt': delivery['receipt'], 'delayMs': delayMs});
  yon_async.Future<Object?> _document(String command, Map<String, Object?> fields) =>
      YonManagedResources.call({'operation': 'document', 'command': command, ...fields});
  yon_async.Future<Object?> _queue(String command, Map<String, Object?> fields) =>
      YonManagedResources.call({'operation': 'queue', 'command': command, ...fields});
}

final class RedisTemplate {
  yon_async.Future<Object?> get(String key) =>
      YonManagedResources.call({'operation': 'redis', 'command': 'get', 'key': key});
  yon_async.Future<Object?> delete(String key) =>
      YonManagedResources.call({'operation': 'redis', 'command': 'delete', 'key': key});
  yon_async.Future<Object?> set(String key, Object? value, int ttlSeconds) => YonManagedResources.call(
      {'operation': 'redis', 'command': 'set', 'key': key, 'value': value, 'ttlSeconds': ttlSeconds});
}

final class YonManagedResources {
  static const int _limit = 1024 * 1024;
  static const Duration scrollTimeout = Duration(seconds: 30);
  static StateError _failed() => StateError('Yon managed resource operation failed.');
  static String _directConnection(Uri endpoint) => 'DIRECT';

  static yon_async.Future<Object?> call(Map<String, Object?> command,
      [Duration timeout = const Duration(seconds: 5)]) async {
    final answer = await exchange(yon_convert.jsonEncode(command), timeout);
    try {
      return yon_convert.jsonDecode(answer);
    } catch (_) {
      throw _failed();
    }
  }

  static yon_async.Future<String> exchange(String command, Duration timeout) async {
    final endpoint = _YonRelayState.resourceEndpoint;
    final token = _YonRelayState.resourceToken;
    if (endpoint == null ||
        !RegExp(r'^http://127\.0\.0\.1:[0-9]{1,5}/$').hasMatch(endpoint) ||
        token == null ||
        !RegExp(r'^[0-9a-f]{64}$').hasMatch(token)) {
      throw _failed();
    }
    final port = int.parse(endpoint.substring(17, endpoint.length - 1));
    if (port < 1 || port > 65535) throw _failed();
    final body = yon_convert.utf8.encode(command);
    if (body.length > _limit) throw _failed();
    final client = yon_io.HttpClient()
      ..connectionTimeout = timeout
      ..findProxy = _directConnection;
    try {
      return await () async {
        final request = await client.post('127.0.0.1', port, '/');
        request.headers
          ..set('authorization', 'Bearer $token')
          ..set('content-type', 'application/json')
          ..contentLength = body.length;
        request.add(body);
        final response = await request.close();
        if (response.statusCode != 200 ||
            response.headers.value('transfer-encoding') != null ||
            response.contentLength < 0 ||
            response.contentLength > _limit) {
          throw _failed();
        }
        final bytes = <int>[];
        await for (final chunk in response) {
          bytes.addAll(chunk);
          if (bytes.length > _limit) throw _failed();
        }
        return yon_convert.utf8.decode(bytes);
      }()
          .timeout(timeout);
    } catch (_) {
      throw _failed();
    } finally {
      client.close(force: true);
    }
  }
}

abstract final class YonGateway {
  static const Duration _answerMargin = Duration(seconds: 1);

  static yon_async.Future<String> call(String key, String arguments) async {
    final body = '{"operation":"gateway","key":${yon_convert.jsonEncode(key)},"arguments":$arguments}';
    if (yon_convert.utf8.encode(body).length > YonManagedResources._limit) return _failure('invalid');
    final timeLeft = _YonRelayState.timeLeftMilliseconds();
    if (timeLeft <= 0) return _failure('timeout');
    try {
      return await YonManagedResources.exchange(body, Duration(milliseconds: timeLeft) + _answerMargin);
    } catch (_) {
      if (_YonRelayState.timeLeftMilliseconds() <= 0) return _failure('timeout');
      return _failure('adapter');
    }
  }

  static String _failure(String kind) => '{"error":"$kind"}';
}

final class __YonFrame {
  __YonFrame(Object? parsed) : _raw = parsed is Map<String, Object?> ? parsed : const {};
  final Map<String, Object?> _raw;

  String method() => '${_raw['method'] ?? ''}';

  String route() => '${_raw['route'] ?? ''}';

  String parameter(String name) {
    final parameters = _raw['parameters'];
    return parameters is Map ? '${parameters[name] ?? ''}' : '';
  }

  String body() {
    final body = _raw['body'];
    return body is Map ? '${body['data'] ?? ''}' : '';
  }

  Map<String, Object?> raw() => _raw;

  List<Object?> arguments() {
    final arguments = _raw['arguments'];
    return arguments is List ? arguments.cast<Object?>() : const [];
  }

  Map<String, List<String>> headers() {
    final headers = _raw['headers'];
    if (headers is! Map) return const {};
    return {for (final entry in headers.entries) '${entry.key}': [for (final value in entry.value as List) '$value']};
  }

  String target() => '${_raw['target'] ?? route()}';
}

yon_shelf.Request _yonShelfRequest(__YonFrame frame) {
  final headers = frame.headers();
  final host = headers['host']?.first ?? 'localhost';
  return yon_shelf.Request(frame.method(), Uri.parse('http://$host${frame.target()}'),
      headers: headers, body: frame.body());
}

yon_async.Future<__YonAnswer> _yonShelfAnswer(yon_shelf.Response response) async {
  final body = await response.readAsString();
  final headers = {for (final entry in response.headersAll.entries) entry.key.toLowerCase(): entry.value};
  return __YonAnswer.json(body)._replaceHeaders(headers).status(response.statusCode);
}

yon_shelf.Response _yonShelfResponse(__YonAnswer answer) =>
    yon_shelf.Response(answer._status, body: answer._body, headers: answer._headers);

final class __YonAnswer {
  __YonAnswer._(this._status, this._body);
  static const int _checkRejectedStatus = 422;
  int _status;
  final String _body;
  final Map<String, List<String>> _headers = {};
  List<Object?>? _violations;

  static __YonAnswer json(String body) =>
      __YonAnswer._(200, body).._headers['content-type'] = ['application/json'];

  static __YonAnswer empty(int status) => __YonAnswer._(status, '');

  static __YonAnswer rejected(List<Object?> violations) =>
      __YonAnswer._(_checkRejectedStatus, '').._violations = violations;

  __YonAnswer status(int value) {
    _status = value;
    return this;
  }

  __YonAnswer header(String name, String value) {
    (_headers[name] ??= []).add(value);
    return this;
  }

  __YonAnswer _replaceHeaders(Map<String, List<String>> replacement) {
    _headers
      ..clear()
      ..addAll(replacement);
    return this;
  }

  String envelope() => yon_convert.jsonEncode({
        'status': _status,
        'headers': _headers,
        'body': _body,
        if (_violations != null) 'violations': _violations,
      });
}

abstract final class _YonRelayLimits {
  static const int stdoutBytes = 16 * 1024 * 1024;
  static const int stderrBytes = 64 * 1024;
}

abstract final class _YonRelayState {
  static const int defaultDeadlineMilliseconds = 30000;
  static const String failureMessage = 'Component invocation failed.';
  static final int Function() _elapsedMilliseconds = _startClock();
  static _YonRequestClock _clock = _YonRequestClock(0);
  static int? _renewedAt;
  static bool _started = false;
  static bool _captured = false;
  static Map<String, String> environment = const {};
  static String requestId = 'unknown';
  static String? resourceEndpoint;
  static String? resourceToken;

  static int get _deadlineMilliseconds => _clock.deadlineMilliseconds;

  static set _deadlineMilliseconds(int value) => _clock.deadlineMilliseconds = value;

  static int Function() _startClock() {
    final clock = Stopwatch()..start();
    return () => clock.elapsedMilliseconds;
  }

  static void captureStart() {
    if (_started) return;
    _started = true;
    _elapsedMilliseconds();
    environment = _relayEnvironment(yon_io.Platform.environment);
    resourceEndpoint = yon_io.Platform.environment['YON_RESOURCE_ENDPOINT'];
    resourceToken = yon_io.Platform.environment['YON_RESOURCE_TOKEN'];
  }

  static Map<String, String> _relayEnvironment(Map<String, String> environment) {
    final kept = <String, String>{};
    environment.forEach((name, value) {
      final upper = name.toUpperCase();
      if (upper == 'YON_APPLICATION_JSON' || upper == 'YON_HANDLER_WARM' || upper.startsWith('YON_RESOURCE_')) {
        return;
      }
      kept[name] = value;
    });
    return kept;
  }

  static void startRequestClock() {
    _clock = _YonRequestClock(_elapsedMilliseconds());
  }

  static void beginRequest(Map<String, Object?> raw) {
    final resources = raw.remove('resources');
    final endpoint = resources is Map ? resources['endpoint'] : null;
    final token = resources is Map ? resources['token'] : null;
    resourceEndpoint = endpoint is String ? endpoint : null;
    resourceToken = token is String ? token : null;
    captureInvocation(raw);
  }

  static void endRequest() {
    _captured = false;
    _renewedAt = null;
    requestId = 'unknown';
    resourceEndpoint = null;
    resourceToken = null;
  }

  static void captureInvocation(Map<String, Object?> raw) {
    if (_captured) return;
    _captured = true;
    final id = raw['request_id'];
    if (id is String) requestId = id;
    _deadlineMilliseconds = _requestedDeadline(raw);
  }

  static int _requestedDeadline(Map<String, Object?> raw) {
    final requested = raw['deadline_ms'];
    if (requested is! num || !requested.isFinite) return defaultDeadlineMilliseconds;
    return requested.toInt();
  }

  static void renew() {
    _captured = true;
    _renewedAt = _elapsedMilliseconds();
  }

  static int timeLeftMilliseconds() {
    if (!_captured) return 0;
    final renewedAt = _renewedAt;
    if (renewedAt != null) return renewedAt + defaultDeadlineMilliseconds - _elapsedMilliseconds();
    return _clock.startedAt + _deadlineMilliseconds - _elapsedMilliseconds();
  }
}

final class _YonRequestClock {
  _YonRequestClock(this.startedAt);

  final int startedAt;
  late final int deadlineMilliseconds;
}

final class _YonRelayOutput {
  _YonRelayOutput(yon_async.Stream<List<int>> pipe, this._limit) {
    _subscription = pipe.listen(_keep, onError: _fail, onDone: _finish, cancelOnError: true);
  }

  final int _limit;
  final List<int> bytes = [];
  bool overflow = false;
  final yon_async.Completer<void> _done = yon_async.Completer<void>();
  late final yon_async.StreamSubscription<List<int>> _subscription;

  yon_async.Future<void> get done => _done.future;

  void _keep(List<int> chunk) {
    final remaining = _limit - bytes.length;
    if (chunk.length > remaining) overflow = true;
    if (remaining <= 0) return;
    bytes.addAll(chunk.take(remaining));
  }

  void _fail(Object error) {
    overflow = true;
    _finish();
  }

  void _finish() {
    if (!_done.isCompleted) _done.complete();
  }

  void abandon() {
    _subscription.cancel();
    _finish();
  }
}

__YonAnswer _yonRelayFailure(String category) {
  yon_io.stderr.writeln(yon_convert.jsonEncode({
    'event': 'handler.relay_failed',
    'request_id': _YonRelayState.requestId,
    'category': category,
  }));
  return __YonAnswer.json(yon_convert.jsonEncode({'error': _YonRelayState.failureMessage})).status(502);
}

__YonAnswer _yonRelayResponse(Map envelope) {
  var response = __YonAnswer.json('${envelope['body'] ?? ''}');
  final status = envelope['status'];
  if (status is num) response = response.status(status.toInt());
  final written = envelope['headers'];
  if (written is Map && written.isNotEmpty) {
    response = response._replaceHeaders({});
    written.forEach((name, values) {
      for (final value in values is List ? values : [values]) {
        response = response.header('$name', '$value');
      }
    });
  }
  return response;
}

yon_async.Future<void> _yonSendRequest(yon_io.IOSink stdin, List<int> payload) async {
  try {
    stdin.add(payload);
    await stdin.close();
  } catch (_) {}
}

yon_async.Future<bool> _yonFinishesInTime(yon_async.Future<Object?> work) async {
  try {
    await work.timeout(Duration(milliseconds: _YonRelayState.timeLeftMilliseconds()));
    return true;
  } on yon_async.TimeoutException {
    return false;
  }
}

__YonAnswer _yonRelayResult(int code, _YonRelayOutput stdout, _YonRelayOutput stderr) {
  if (stdout.overflow || stderr.overflow) return _yonRelayFailure('overflow');
  if (code != 0) return _yonRelayFailure('exit');
  final Object? envelope;
  try {
    envelope = yon_convert.jsonDecode(yon_convert.utf8.decode(stdout.bytes));
  } on FormatException {
    return _yonRelayFailure('protocol');
  }
  if (envelope is! Map) return _yonRelayFailure('protocol');
  return _yonRelayResponse(envelope);
}

yon_async.Future<yon_shelf.Response> relay(List<String> command, yon_shelf.Request request) async =>
    _yonShelfResponse(await _yonRelayAnswer(command, request));

yon_async.Future<Map<String, Object?>> _yonRelayMessage(yon_shelf.Request request) async => {
      'route': request.requestedUri.hasQuery
          ? '${request.requestedUri.path}?${request.requestedUri.query}'
          : request.requestedUri.path,
      'method': request.method,
      'headers': request.headersAll,
      'body': {'encoding': 'utf8', 'data': await request.readAsString()},
    };

yon_async.Future<__YonAnswer> _yonRelayAnswer(List<String> command, yon_shelf.Request request) async {
  if (command.isEmpty) return _yonRelayFailure('start');
  if (_YonRelayState.timeLeftMilliseconds() <= 0) return _yonRelayFailure('timeout');
  final payload = yon_convert.utf8.encode(yon_convert.jsonEncode(await _yonRelayMessage(request)));
  final yon_io.Process child;
  try {
    child = await yon_io.Process.start(command.first, command.sublist(1),
        environment: _YonRelayState.environment, includeParentEnvironment: false);
  } on yon_io.ProcessException {
    return _yonRelayFailure('start');
  }
  final stdout = _YonRelayOutput(child.stdout, _YonRelayLimits.stdoutBytes);
  final stderr = _YonRelayOutput(child.stderr, _YonRelayLimits.stderrBytes);
  yon_async.unawaited(_yonSendRequest(child.stdin, payload));
  final finished = yon_async.Future.wait<Object?>([child.exitCode, stdout.done, stderr.done]);
  if (!await _yonFinishesInTime(finished)) {
    child.kill(yon_io.ProcessSignal.sigkill);
    stdout.abandon();
    stderr.abandon();
    return _yonRelayFailure('timeout');
  }
  return _yonRelayResult(await child.exitCode, stdout, stderr);
}

String? _yonMiddlewarePhase(__YonFrame request) {
  switch (request.raw()['operation']) {
    case 'middleware.before':
      return 'before';
    case 'middleware.after':
      return 'after';
  }
  return null;
}

Object? _yonCall(Object? Function() method) => method();

Object? _yonDispatch(__YON_HANDLER__ handler, __YonFrame request) {
  final phase = _yonMiddlewarePhase(request);
  final delivery = request.raw()['operation'] == 'event.deliver';
  switch (delivery ? '__yonDeliver' : phase ?? request.method()) {
//__YON_DISPATCH__
    default:
      return __YonAnswer.empty(delivery ? 404 : phase == null ? 405 : 204);
  }
}

yon_async.Future<void> main() async {
  _YonRelayState.captureStart();
  if (_YonWarm.enabled()) return _YonWarm.serve();
  final bytes = <int>[];
  if (YonSocket.framed()) {
    bytes.addAll(yon_convert.utf8.encode(await YonSocket.readFrame() ?? ''));
  } else {
    await for (final chunk in yon_io.stdin) {
      bytes.addAll(chunk);
      if (bytes.length > 16 * 1024 * 1024) return;
    }
  }
  final __YonFrame request;
  try {
    request = __YonFrame(yon_convert.jsonDecode(yon_convert.utf8.decode(bytes)));
  } on FormatException {
    return;
  }
  _YonRelayState.captureInvocation(request.raw());
  final __YonAnswer? response;
  try {
    response = await _yonAnswer(request);
  } catch (_) {
    yon_io.exitCode = 1;
    return;
  }
  if (response == null) return;
  yon_io.stdout.write(response.envelope());
  await yon_io.stdout.flush();
}

yon_async.Future<__YonAnswer?> _yonAnswer(__YonFrame request) async {
  try {
    var result = _yonDispatch(__YON_HANDLER__(), request);
    if (result is yon_async.Future) result = await result;
    if (result is yon_async.Stream) {
      final id = '${request.raw()['request_id'] ?? ''}';
      if (await _yonStreamEvents(id, result) && _YonWarm.enabled()) await YonStream.end(id);
      return null;
    }
    if (result is yon_shelf.Response) return await _yonShelfAnswer(result);
    if (result is __YonAnswer) return result;
    if (result == null && _yonMiddlewarePhase(request) != null) return __YonAnswer.empty(204);
    return __YonAnswer.json(yon_convert.jsonEncode(result));
  } catch (failure) {
    final violations = _yonViolations(failure);
    if (violations == null) rethrow;
    return __YonAnswer.rejected(violations);
  }
}

yon_async.Future<bool> _yonStreamEvents(String id, yon_async.Stream<Object?> events) async {
  try {
    await for (final event in events) {
      if (!await YonStream.event(id, event)) return false;
    }
  } catch (_) {
    await YonStream.failure(id);
    return false;
  }
  return true;
}

abstract final class _YonWarm {
  static bool enabled() => yon_io.Platform.environment['YON_HANDLER_WARM'] == '1';

  static yon_async.Future<void> serve() async {
    await YonSocket._write(yon_convert.jsonEncode({'protocol_version': 1, 'kind': 'ready'}));
    while (true) {
      final request = await _nextRequest();
      final response = await _answer(request);
      if (response != null) await YonSocket._write(response.envelope());
      _YonRelayState.endRequest();
    }
  }

  static yon_async.Future<__YonFrame> _nextRequest() async {
    if (await YonSocket._inputEnded()) return _stop(0);
    _YonRelayState.startRequestClock();
    final fields = await _requestFields();
    _YonRelayState.beginRequest(fields);
    return __YonFrame(fields);
  }

  static yon_async.Future<Map<String, Object?>> _requestFields() async {
    final Object? parsed;
    try {
      parsed = yon_convert.jsonDecode(await YonSocket.readFrame() ?? '');
    } on FormatException {
      return _stop(70);
    }
    if (parsed is! Map<String, Object?>) return _stop(70);
    if (parsed['kind'] != 'request' || parsed['operation'] == 'socket.open') return _stop(70);
    return parsed;
  }

  static yon_async.Future<__YonAnswer?> _answer(__YonFrame request) async {
    try {
      return await _yonAnswer(request);
    } catch (_) {
      return _stop(1);
    }
  }

  static yon_async.Future<Never> _stop(int code) async {
    await yon_io.stderr.flush();
    yon_io.exit(code);
  }
}

List<Object?>? _yonViolations(Object failure) {
  try {
    final Object? found = (failure as dynamic).violations;
    return found is List ? found : null;
  } on NoSuchMethodError {
    return null;
  }
}

abstract interface class YonSocketEndpoint {
  dynamic open(SocketSession session);
  dynamic message(SocketSession session, Object? value);
  dynamic close(SocketSession session);
  dynamic error(SocketSession session, Object error);
}

final class SocketPeer {
  SocketPeer._(this._session, this.id);

  final SocketSession _session;
  final String id;

  yon_async.Future<void> send(Object? message) => _session._send(message, id);
}

final class SocketSession {
  SocketSession._(this.id, this._encode);

  final String id;
  final Object? Function(Object?) _encode;
  final List<Object?> _inbox = [];
  bool _closed = false;
  List<String>? _sessions;

  yon_async.Future<void> send(Object? message) => _send(message, null);

  yon_async.Future<void> close() {
    _closed = true;
    return _write({'kind': 'close'});
  }

  yon_async.Future<List<SocketPeer>> openSessions() async {
    _sessions = null;
    await _write({'kind': 'sessions'});
    while (_sessions == null && !_closed) {
      await _receive();
    }
    return [for (final peer in _sessions ?? const <String>[]) SocketPeer._(this, peer)];
  }

  yon_async.Future<void> _send(Object? message, String? to) {
    final frame = <String, Object?>{
      'kind': 'event',
      'body': {'encoding': 'utf8', 'data': yon_convert.jsonEncode(_encode(message))},
    };
    if (to != null) frame['to'] = to;
    return _write(frame);
  }

  yon_async.Future<Object?> _next() async {
    while (_inbox.isEmpty && !_closed) {
      await _receive();
    }
    return _inbox.isEmpty ? YonSocket._ended : _inbox.removeAt(0);
  }

  yon_async.Future<void> _receive() async {
    final raw = await YonSocket.readFrame();
    final frame = raw == null ? null : yon_convert.jsonDecode(raw);
    if (frame is! Map) {
      _closed = true;
      return;
    }
    if (frame['request_id'] != id) yon_io.exit(70);
    switch (frame['kind']) {
      case 'message':
        final body = frame['body'];
        _inbox.add(yon_convert.jsonDecode(body is Map ? '${body['data']}' : 'null'));
      case 'sessions':
        final list = frame['sessions'];
        _sessions = list is List ? [for (final peer in list) '$peer'] : <String>[];
      default:
        _closed = true;
    }
  }

  yon_async.Future<void> _write(Map<String, Object?> fields) => YonSocket._write(
      yon_convert.jsonEncode({...fields, 'protocol_version': 1, 'request_id': id}));
}

abstract final class YonSocket {
  static const int _maxFrameBytes = 16 * 1024 * 1024;
  static final Object _ended = Object();
  static final yon_async.StreamIterator<List<int>> _input = yon_async.StreamIterator(yon_io.stdin);
  static final List<int> _buffer = <int>[];
  static yon_async.Future<void> _pending = yon_async.Future<void>.value();

  static bool framed() => yon_io.Platform.environment['YON_SOCKET_FRAMES'] == '1';

  static yon_async.Future<List<int>?> _take(int length) async {
    while (_buffer.length < length) {
      if (!await _input.moveNext()) return null;
      _buffer.addAll(_input.current);
    }
    final taken = _buffer.sublist(0, length);
    _buffer.removeRange(0, length);
    return taken;
  }

  static yon_async.Future<bool> _inputEnded() async {
    while (_buffer.isEmpty) {
      if (!await _input.moveNext()) return true;
      _buffer.addAll(_input.current);
    }
    return false;
  }

  static yon_async.Future<String?> readFrame() async {
    final prefix = await _take(4);
    if (prefix == null) return null;
    final length = prefix[0] << 24 | prefix[1] << 16 | prefix[2] << 8 | prefix[3];
    if (length > _maxFrameBytes) yon_io.exit(70);
    final payload = await _take(length);
    return payload == null ? null : yon_convert.utf8.decode(payload);
  }

  static yon_async.Future<void> _write(String text) {
    final payload = yon_convert.utf8.encode(text);
    final length = payload.length;
    return _pending = _pending.then((_) async {
      yon_io.stdout.add([length >> 24 & 255, length >> 16 & 255, length >> 8 & 255, length & 255]);
      yon_io.stdout.add(payload);
      await yon_io.stdout.flush();
    });
  }

  static yon_async.Future<void> _call(dynamic Function() action) async {
    _YonRelayState.renew();
    final Object? result = action();
    if (result is yon_async.Future) await result;
  }

  static yon_async.Future<void> _attempt(
      YonSocketEndpoint endpoint, SocketSession session, dynamic Function() action) async {
    try {
      await _call(action);
    } catch (failure) {
      yon_io.stderr.writeln(yon_convert.jsonEncode({
        'event': 'handler.socket_failed',
        'request_id': session.id,
        'message': '$failure',
      }));
      try {
        await _call(() => endpoint.error(session, failure));
      } catch (_) {}
    }
  }

  static yon_async.Future<Object?> run(
      YonSocketEndpoint endpoint, __YonFrame request, Object? Function(Object?) encode) async {
    final session = SocketSession._('${request.raw()['request_id'] ?? ''}', encode);
    await _attempt(endpoint, session, () => endpoint.open(session));
    while (true) {
      final message = await session._next();
      if (identical(message, _ended)) break;
      await _attempt(endpoint, session, () => endpoint.message(session, message));
    }
    try {
      await _call(() => endpoint.close(session));
    } catch (_) {}
    await _pending;
    yon_io.exit(0);
  }
}

final class YonStream {
  static const _maxFrameBytes = 16 * 1024 * 1024;

  static yon_async.Future<bool> event(String requestId, Object? value) => _frame(requestId, {
        'protocol_version': 1,
        'kind': 'event',
        'request_id': requestId,
        'body': {'encoding': 'utf8', 'data': yon_convert.jsonEncode(value)},
      });

  static yon_async.Future<void> failure(String requestId) =>
      _frame(requestId, _error(requestId, 'TY2204', 'The handler stream failed.'));

  static yon_async.Future<void> end(String requestId) =>
      _frame(requestId, {'protocol_version': 1, 'kind': 'end', 'request_id': requestId});

  static Map<String, Object?> _error(String requestId, String code, String message) => {
        'protocol_version': 1,
        'kind': 'response',
        'request_id': requestId,
        'status': 500,
        'headers': <String, Object?>{},
        'error': {'code': code, 'message': message, 'retryable': false},
      };

  static yon_async.Future<bool> _frame(String requestId, Map<String, Object?> frame) async {
    final payload = yon_convert.utf8.encode(yon_convert.jsonEncode(frame));
    if (payload.length > _maxFrameBytes) {
      await _write(yon_convert.utf8.encode(yon_convert.jsonEncode(
          _error(requestId, 'TY2203', 'Streamed event exceeds the protocol frame limit.'))));
      return false;
    }
    await _write(payload);
    return true;
  }

  static yon_async.Future<void> _write(List<int> payload) async {
    final length = payload.length;
    yon_io.stdout.add([length >> 24 & 255, length >> 16 & 255, length >> 8 & 255, length & 255]);
    yon_io.stdout.add(payload);
    await yon_io.stdout.flush();
  }
}
