#[Service]
class LedgerService
  #[Needs]
  # @return [LedgerAmountsDelegate]
  attr_reader :ledger_amounts_delegate

  # @return [Integer]
  def subtotal
    ledger_amounts_delegate.subtotal
  end
end
