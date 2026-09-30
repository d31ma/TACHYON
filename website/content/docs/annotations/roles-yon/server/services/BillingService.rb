#[Service]
class BillingService
  SUBTOTAL = 40

  #[Needs]
  # @return [BillingHelper]
  attr_reader :billing_helper

  # @return [Integer]
  def total
    billing_helper.settle(SUBTOTAL)
  end
end
