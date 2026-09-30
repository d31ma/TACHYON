#[Helper]
class TaxHelper
  #[Needs]
  # @return [TaxRateHelper]
  attr_reader :tax_rate_helper

  # @param subtotal [Integer]
  # @return [Integer]
  def add_tax(subtotal)
    tax = tax_rate_helper.rate
    subtotal + tax
  end
end
