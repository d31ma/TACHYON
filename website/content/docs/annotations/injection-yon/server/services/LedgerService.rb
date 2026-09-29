#[Service]
class LedgerService
  #[Needs]
  # @return [LedgerAmountsHelper]
  attr_reader :ledger_amounts_helper

  # @return [Integer]
  def subtotal
    ledger_amounts_helper.subtotal
  end
end
