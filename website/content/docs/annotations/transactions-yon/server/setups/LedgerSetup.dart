@Setup
class LedgerSetup {
  static BehaviorAdapters configure(BehaviorExecution execution) {
    return BehaviorAdapters(
      entities: {'ledger': BehaviorEntityAdapter(execute: LedgerAdapterHelper.execute)},
      authorizeEntity: (command, execution) async => true,
    );
  }
}
