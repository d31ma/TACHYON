#[HTTPClient("/api")]
class OrderClient
  # @param order [Hash]
  # @return [OrderInput]
  #[PostRequest("/orders")]
  def send_order(
    #[RequestBody]
    order
  )
  end
end
