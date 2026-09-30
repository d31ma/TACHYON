#[Service]
class BillingService
  # @return [Integer]
  attr_reader :total

  def initialize
    @total = 42
  end
end
