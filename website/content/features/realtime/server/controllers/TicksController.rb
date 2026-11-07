#[RequestRoute("/ticks")]
#[Public]
class TicksController
  # @return [void]
  #[StreamRoute]
  def stream_ticks
    countdown(3) { |tick| yield tick }
  end

  private

  # @param start [Integer]
  # @return [Integer]
  def countdown(start)
    start.downto(1) do |level|
      yield(Tick.new(sku: "ANVIL-1", level: level))
      sleep(1)
    end
  end
end
