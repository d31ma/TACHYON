#[Service]
class BasketService
  # @return [Integer]
  attr_reader :count
  # @return [String]
  attr_reader :label

  # @return [void]
  #[OnStart]
  def start
    @count = 6
    @label = "Ruby"
  end

  # @return [Integer]
  def doubled
    @count * 2
  end
end
