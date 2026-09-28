import 'models/LedgerReport.dart';

@RequestRoute("/ledger")
@Public
class LedgerController {
  @GetRoute
  Future<LedgerReport> getLedger() async {
    final context = Behaviors.create(LedgerSetup, partition: 'ledger', signal: BehaviorSignal());
    final entries = EntryStore(context);
    final settled = await entries.settle('open');
    context.dispose();
    return LedgerReport(settled, LedgerAdapterHelper.commands);
  }
}
