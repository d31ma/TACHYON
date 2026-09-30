#[RequestRoute("/checkout")]
#[Public]
class CheckoutController
  #[Needs]
  # @return [CheckoutService]
  attr_reader :checkout_service

  # @return [Receipt]
  #[GetRoute]
  def get_checkout
    Receipt.new(total: checkout_service.total)
  end
end
