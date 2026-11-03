#[Service]
class PriceService
  CURRENCY = "EUR"

  # @param amount [Integer]
  # @param count [Integer]
  # @return [Price]
  def price_for(amount, count)
    Price.new(amount: amount * count, currency: CURRENCY)
  end
end
