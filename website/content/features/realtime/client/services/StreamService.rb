#[Service]
class StreamService
  #[Needs]
  # @return [TicksClient]
  attr_reader :ticks_client

  # @return [Array<Tick>]
  attr_reader :ticks

  def initialize
    @ticks = []
  end

  # @param tick [Tick]
  # @return [void]
  #[OnEvent]
  def add_tick(tick)
    @ticks = [*@ticks.last(tick_limit - 1), tick]
  end

  private

  # @return [Integer]
  def tick_limit
    128
  end
end
