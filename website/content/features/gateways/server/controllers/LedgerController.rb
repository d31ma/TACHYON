#[RequestRoute("/ledger")]
#[Public]
class LedgerController
  #[Needs]
  # @return [LedgerClient]
  attr_reader :ledger_client

  # @return [Balance]
  #[GetRoute]
  def get_ledger
    Balance.new(balance: ledger_client.balance("ACME-7"), forecast: ledger_client.forecast(1))
  end
end
