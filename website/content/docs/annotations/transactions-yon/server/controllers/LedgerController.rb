#[RequestRoute("/ledger")]
#[Public]
class LedgerController
  #[Needs]
  # @return [LedgerService]
  attr_reader :ledger_service

  # @return [LedgerReport]
  #[GetRoute]
  def get_ledger
    LedgerReport.new(settled: ledger_service.settle("open"), commands: LedgerAdapterHelper.commands)
  end
end
