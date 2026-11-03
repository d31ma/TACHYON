#[Service]
class LiveTotalService
  # @return [Integer]
  attr_reader :seen

  def initialize
    @seen = 0
  end

  # @param changed [CartTotal]
  # @return [void]
  #[OnEvent({"replay":true})]
  def changed(changed)
    @seen = changed.total
  end
end
