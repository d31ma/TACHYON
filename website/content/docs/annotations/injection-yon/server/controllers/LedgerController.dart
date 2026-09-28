import 'models/Ledger.dart';

@RequestRoute("/ledger")
@Public
class LedgerController {
  @Needs
  @Named("LedgerService")
  final LedgerService service;
  @Needs
  @Named("LedgerHelper")
  final LedgerHelper settlement;

  @GetRoute
  Ledger getLedger() {
    final subtotal = service.subtotal();
    final total = settlement.settle(subtotal);
    return Ledger(total);
  }
}
