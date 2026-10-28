#[Service]
class CheckoutService
  #[Needs]
  # @return [TaxDelegate]
  attr_reader :helper

  # @return [Integer]
  def total
    helper.add_tax(subtotal)
  end

  private

  # @return [Integer]
  def subtotal
    40
  end
end
