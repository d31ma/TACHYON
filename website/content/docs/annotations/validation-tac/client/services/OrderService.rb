#[Service]
class OrderService
  # @return [String]
  attr_reader :status

  def initialize
    @status = "ready"
  end

  # @param order [Hash]
  # @return [Hash]
  def accept(
    #[Check(OrderInput)]
    order
  )
    order
  end
end
