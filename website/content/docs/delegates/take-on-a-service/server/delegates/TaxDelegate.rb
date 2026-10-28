#[Delegate]
class TaxDelegate
  #[Needs]
  # @return [TaxService]
  attr_reader :tax_service

  # @param region [String]
  # @return [Integer]
  def rate(region)
    tax_service.rate(region) * 2
  end

  # @param count [Integer]
  # @param extra [Integer]
  # @return [Integer]
  def total(count, extra)
    tax_service.total(count) + extra
  end
end
