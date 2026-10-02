@TestSetup
class HelloTestSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(http: {
      'greeting': BehaviorHttpAdapter(exchange: (request, running) async => {
        'message': 'Hi',
      }),
    });
  }
}
