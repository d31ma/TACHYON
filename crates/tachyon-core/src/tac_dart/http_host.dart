Future<String> tachyonHttpHost(
  String request,
  void Function() Function(void Function()) watch,
) async {
  final symbol = (globalContext['Symbol'] as JSObject).callMethod<JSAny>(
    'for'.toJS,
    'tachyon.tac.http'.toJS,
  );
  final http = globalContext.getProperty<JSObject?>(symbol);
  if (http == null) return '{"error":{"code":"HTTP_MISSING"}}';
  final ticket = http.callMethod<JSObject>('start'.toJS, request.toJS);
  final stop = watch(() {
    ticket.callMethod<JSAny?>('cancel'.toJS);
  });
  try {
    final answer = ticket.getProperty<JSPromise<JSString>>('answer'.toJS);
    return (await answer.toDart).toDart;
  } finally {
    stop();
  }
}
