#[RequestRoute("/price")]
#[Public]
class PriceController
  #[Needs]
  # @return [PriceService]
  attr_reader :price_service

  # @return [Price]
  #[GetRoute]
  def get_price
    price_service.price_for(20, 3)
  end
end
