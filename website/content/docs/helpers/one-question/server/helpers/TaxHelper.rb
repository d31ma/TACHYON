#[Helper]
class TaxHelper
  #[Needs]
  # @return [TaxRateHelper]
  attr_reader :rates

  # @param subtotal [Integer]
  # @return [Integer]
  def add_tax(subtotal)
    tax = rates.rate
    subtotal + tax
  end
end
