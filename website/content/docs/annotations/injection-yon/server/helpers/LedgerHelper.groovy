@Helper
@Lifetime("transient")
class LedgerHelper {
  @Needs
  LedgerStore repository

  int settle(int subtotal) {
    int surcharge = repository.surcharge()
    return subtotal + surcharge
  }
}
