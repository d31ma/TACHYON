#[ErrorHandler]
class OrdersErrorHandler
  # @param error [OrderNotFound]
  # @return [Problem]
  #[HandlesError]
  #[ResponseStatus(404)]
  def missing(error)
    Problem.new(code: "order-not-found")
  end
end
