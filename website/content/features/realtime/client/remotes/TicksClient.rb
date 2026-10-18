#[HTTPClient("/ticks")]
class TicksClient
  # @param tick [Tick]
  # @return [void]
  #[StreamRequest]
  def receive(tick)
  end
end
