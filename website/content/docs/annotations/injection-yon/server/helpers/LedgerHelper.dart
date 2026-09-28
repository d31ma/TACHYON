@Helper
@Lifetime("transient")
class LedgerHelper {
  @Needs
  final LedgerStore repository;

  int settle(int subtotal) {
    final surcharge = repository.surcharge();
    return subtotal + surcharge;
  }
}
