import 'models/Balance.dart';

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  final LedgerClient ledgerClient;

  @GetRoute
  Future<Balance> getLedger() async {
    final balance = await ledgerClient.balance('ACME-7');
    final forecast = await ledgerClient.forecast(1);
    return Balance(balance, forecast);
  }
}
