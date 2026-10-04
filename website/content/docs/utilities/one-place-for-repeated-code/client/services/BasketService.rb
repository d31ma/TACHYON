#[Service]
class BasketService
  def initialize
    @total = 1250
  end

  # @return [String]
  def total_label
    PriceUtils.label(@total)
  end
end
