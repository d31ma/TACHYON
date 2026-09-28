@Helper
@Lifetime('transient')
class LedgerHelper {
  @Needs
  repository: LedgerStore;

  settle(subtotal: number): number {
    const surcharge = this.repository.surcharge();
    return subtotal + surcharge;
  }
}
