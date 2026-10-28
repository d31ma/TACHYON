#[Delegate]
#[Lifetime("transient")]
class LedgerDelegate
  #[Needs]
  # @return [LedgerAmountsDelegate]
  attr_reader :ledger_amounts_delegate

  # @param subtotal [Integer]
  # @return [Integer]
  def settle(subtotal)
    subtotal + ledger_amounts_delegate.surcharge
  end
end
