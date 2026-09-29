#[Helper]
#[Lifetime("transient")]
class LedgerHelper
  #[Needs]
  # @return [LedgerAmountsHelper]
  attr_reader :ledger_amounts_helper

  # @param subtotal [Integer]
  # @return [Integer]
  def settle(subtotal)
    subtotal + ledger_amounts_helper.surcharge
  end
end
