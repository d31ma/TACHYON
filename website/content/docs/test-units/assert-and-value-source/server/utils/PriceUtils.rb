class PriceUtils
  def self.with_tax(cents)
    raise ArgumentError, "A price is never negative." if cents.negative?

    (cents * 1.2).round
  end
end
