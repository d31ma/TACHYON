#[Service]
class BasketService
  # @return [Integer]
  attr_reader :count

  def initialize
    @count = 0
  end

  # @return [Integer]
  def doubled
    @count * 2
  end

  # @return [void]
  #[OnStart]
  def start
    @count = 6
  end
end
