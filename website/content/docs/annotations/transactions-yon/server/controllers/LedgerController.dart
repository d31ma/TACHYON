import 'models/LedgerReport.dart';

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  final LedgerService ledger;

  @GetRoute
  Future<LedgerReport> getLedger() async {
    final settled = await ledger.settle('open');
    return LedgerReport(settled, LedgerAdapterHelper.commands);
  }
}
