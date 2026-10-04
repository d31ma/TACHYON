class PriceUtils
  # @param cents [Integer]
  # @return [String]
  def self.label(cents)
    "$#{_dollars(cents)}"
  end

  # @param cents [Integer]
  # @return [String]
  def self._dollars(cents)
    format("%.2f", cents / 100.0)
  end
end
