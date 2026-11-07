import 'models/Ledger.dart';

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  @Named("LedgerService")
  final LedgerService service;
  @Needs
  @Named("LedgerDelegate")
  final LedgerDelegate settlement;

  @GetRoute
  Ledger getLedger() {
    final subtotal = service.subtotal();
    final total = settlement.settle(subtotal);
    return Ledger(total);
  }
}
