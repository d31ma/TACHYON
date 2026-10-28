#[Delegate]
class TaxDelegate
  #[Needs]
  # @return [TaxRateDelegate]
  attr_reader :rates

  # @param subtotal [Integer]
  # @return [Integer]
  def add_tax(subtotal)
    tax = rates.rate
    subtotal + tax
  end
end
