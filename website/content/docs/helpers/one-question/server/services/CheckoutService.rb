#[Service]
class CheckoutService
  #[Needs]
  # @return [TaxHelper]
  attr_reader :tax_helper

  # @return [Integer]
  def total
    tax_helper.add_tax(subtotal)
  end

  private

  # @return [Integer]
  def subtotal
    40
  end
end
