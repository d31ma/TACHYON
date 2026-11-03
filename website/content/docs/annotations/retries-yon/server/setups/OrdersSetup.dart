@Setup
class OrdersSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(authorize: (call, execution) async => true);
  }
}
