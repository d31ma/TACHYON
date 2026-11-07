@Setup
class OverrideSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(http: {
      'override': BehaviorHttpAdapter(exchange: (request, running) async => {
        'message': 'Custom adapter wins',
      }),
    });
  }
}
