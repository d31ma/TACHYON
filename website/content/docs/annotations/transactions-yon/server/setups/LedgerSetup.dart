@Setup
class LedgerSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(
      entities: {'ledger': BehaviorEntityAdapter(execute: LedgerAdapterDelegate.execute)},
      authorizeEntity: (command, execution) async => true,
    );
  }
}
