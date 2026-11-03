#[RequestRoute("/money")]
#[Public]
class MoneyController
  #[Needs]
  # @return [MoneyService]
  attr_reader :money_service

  # @return [Money]
  #[GetRoute]
  def get_money
    Money.new(amount: money_service.amount)
  end
end
