#[Service]
class BadgeService
  # @return [String]
  attr_reader :status

  def initialize
    @status = "ready"
  end
end
