#[Service]
class LedgerService
  #[Needs]
  # @return [LedgerStore]
  attr_reader :ledger_store

  # @return [Integer]
  def subtotal
    ledger_store.subtotal
  end
end
