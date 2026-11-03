#[Service]
class TicketService
  # @return [String]
  attr_reader :ticket
  # @return [Float]
  attr_reader :issued_at

  def initialize
    @clock = Clock.new
    @secure_random = SecureRandom.new
    @ticket = ""
    @issued_at = 0.0
  end

  # @return [void]
  def issue
    @ticket = @secure_random.uuid
    @issued_at = @clock.now_ms
  end
end
