#[RequestRoute("/ticks")]
#[Public]
class TicksController
  # @return [void]
  #[StreamRoute]
  def stream_ticks
    ticks_up_to(3) { |tick| yield tick }
  end

  private

  # @param last [Integer]
  # @return [void]
  def ticks_up_to(last)
    1.upto(last) { |level| yield(Tick.new(sku: "ANVIL-1", level: level)) }
  end
end
