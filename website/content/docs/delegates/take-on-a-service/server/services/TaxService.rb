#[Service]
class TaxService
  # @param region [String]
  # @return [Integer]
  def rate(region)
    2
  end

  # @param count [Integer]
  # @return [Integer]
  def total(count)
    count * 10
  end
end
