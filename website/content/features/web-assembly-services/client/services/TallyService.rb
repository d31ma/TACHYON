#[Service]
class TallyService
  # @return [Integer]
  attr_reader :count

  def initialize
    @count = 0
  end

  # @param amount [Integer]
  # @return [Integer]
  def add(amount)
    @count += amount
  end

  # @param unit [String]
  # @return [String]
  def summary(unit)
    "#{@count} #{unit}"
  end
end
