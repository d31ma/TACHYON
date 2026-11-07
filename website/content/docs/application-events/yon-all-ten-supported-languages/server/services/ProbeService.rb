#[Service]
class ProbeService
  # @return [ProbeReady, nil]
  attr_reader :last

  def initialize
    @last = nil
  end

  # @param ready [ProbeReady]
  # @return [void]
  #[OnEvent]
  def observe(ready)
    @last = ready
  end

  # @return [void]
  #[OnStop]
  def destroy
    @last = nil
  end
end
