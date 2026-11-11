@TestSetup
class OrdersTestSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(http: {
      'orders': BehaviorHttpAdapter(exchange: _echo),
    });
  }

  static Future<Object?> _echo(Object? request, BehaviorExecution execution) async {
    final sent = request as Map;
    if (sent['path'] == '/api/claims/a-7') throw BehaviorError.http(409, {'reason': 'taken', 'holder': 'south'});
    final body = sent['body'];
    return <String, Object?>{
      'method': sent['method'],
      'path': sent['path'],
      'header': (sent['headers'] as Map).values.single,
      'item': body is Map ? body['item'] : 'none',
    };
  }
}
