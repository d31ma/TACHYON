#[Schedule({"fixedDelaySeconds":300})]
class Handler
  #[Needs]
  # @return [LedgerService]
  attr_reader :ledger

  # @return [Integer]
  def POST
    ledger.total
  end
end
