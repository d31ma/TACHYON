#[RequestRoute("/ledger")]
#[Public]
class LedgerController
  #[Needs]
  #[Named("LedgerService")]
  # @return [LedgerService]
  attr_reader :service
  #[Needs]
  #[Named("LedgerHelper")]
  # @return [LedgerHelper]
  attr_reader :settlement

  # @return [Ledger]
  #[GetRoute]
  def get_ledger
    total = settlement.settle(service.subtotal)
    Ledger.new(total: total)
  end
end
