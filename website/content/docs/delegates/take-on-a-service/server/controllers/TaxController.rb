#[RequestRoute("/tax")]
#[Public]
class TaxController
  COUNT = 3

  #[Needs]
  # @return [TaxDelegate]
  attr_reader :tax_delegate

  # @return [TaxAnswer]
  #[GetRoute]
  def get_tax
    TaxAnswer.new(
      rate: tax_delegate.rate("north"),
      total: tax_delegate.total(COUNT),
      extra: tax_delegate.total(COUNT, 4)
    )
  end
end
