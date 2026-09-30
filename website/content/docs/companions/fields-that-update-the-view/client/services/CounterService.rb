#[Service]
class CounterService
  # @return [Integer]
  attr_reader :count
  # @return [Integer]
  #[SessionState]
  attr_accessor :visits
  # @return [String]
  #[LocalState]
  attr_accessor :theme

  def initialize
    @count = 0
    @visits = 0
    @theme = "light"
  end

  # @return [void]
  def increment
    @count += 1
  end
end
