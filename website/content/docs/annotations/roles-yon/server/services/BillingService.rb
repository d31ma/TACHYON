#[Service]
class BillingService
  SUBTOTAL = 40

  #[Needs]
  # @return [BillingDelegate]
  attr_reader :billing_delegate

  # @return [Integer]
  def total
    billing_delegate.settle(SUBTOTAL)
  end
end
