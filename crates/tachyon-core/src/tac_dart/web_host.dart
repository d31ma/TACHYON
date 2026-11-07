final class TacWebHost {
  static Object? call(
    String api,
    String call, [
    List<Object?> arguments = const [],
  ]) {
    final request = jsonEncode({'api': api, 'call': call, 'args': arguments});
    final answer = jsonDecode(_send(request)) as Map<String, Object?>;
    return _valueOf(answer);
  }

  static String _send(String request) {
    final symbol = (globalContext['Symbol'] as JSObject).callMethod<JSAny>(
      'for'.toJS,
      'tachyon.tac.web'.toJS,
    );
    final web = globalContext.getProperty<JSObject?>(symbol);
    if (web == null)
      return '{"error":"This browser API needs the page, and this service runs without one."}';
    return web.callMethod<JSString>('host'.toJS, request.toJS).toDart;
  }

  static Object? _valueOf(Map<String, Object?> answer) {
    final error = answer['error'];
    if (error is String) throw StateError(error);
    return answer['value'];
  }
}
