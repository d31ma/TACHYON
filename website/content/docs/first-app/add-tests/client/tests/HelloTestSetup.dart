@TestSetup
class HelloTestSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(http: {
      'greeting': BehaviorHttpAdapter(exchange: _greeting),
    });
  }

  static Future<Object?> _greeting(Object? request, BehaviorExecution running) async {
    return <String, Object?>{'message': 'Hi'};
  }
}
