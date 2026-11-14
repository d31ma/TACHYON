#[Service]
class OrderService
  # @param order [Hash]
  # @return [Hash]
  def accept(
    #[Check(OrderInput)]
    order
  )
    order
  end
end
