#[Helper]
#[Lifetime("transient")]
class LedgerHelper
  #[Needs]
  # @return [LedgerStore]
  attr_reader :ledger_store

  # @param subtotal [Integer]
  # @return [Integer]
  def settle(subtotal)
    subtotal + ledger_store.surcharge
  end
end
