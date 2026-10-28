#[RequestRoute("/checkout")]
#[Public]
class CheckoutController
  #[Needs]
  # @return [CheckoutService]
  attr_reader :service

  # @return [Receipt]
  #[GetRoute]
  def get_checkout
    Receipt.new(total: service.total)
  end
end
