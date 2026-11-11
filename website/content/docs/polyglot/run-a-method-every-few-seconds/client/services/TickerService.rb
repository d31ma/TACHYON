#[Service]
class TickerService
  # @return [Integer]
  attr_reader :seconds

  def initialize
    @seconds = 0
  end

  #[Every("1s")]
  def count
    @seconds += 1
  end
end
