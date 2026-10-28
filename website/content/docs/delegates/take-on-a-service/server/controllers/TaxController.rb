#[RequestRoute("/tax")]
#[Public]
class TaxController
  #[Needs]
  # @return [TaxDelegate]
  attr_reader :tax_delegate

  # @return [TaxAnswer]
  #[GetRoute]
  def get_tax
    TaxAnswer.new(
      rate: tax_delegate.rate("north"),
      total: tax_delegate.total(3),
      extra: tax_delegate.total(3, 4)
    )
  end
end
