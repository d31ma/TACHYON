#[RequestRoute("/billing")]
#[Public]
class BillingController
  #[Needs]
  # @return [BillingService]
  attr_reader :billing_service

  # @return [Bill]
  #[GetRoute]
  def get_billing
    Bill.new(total: billing_service.total)
  end
end
