#[Service]
class LedgerService
  #[Needs]
  # @return [EntryStore]
  attr_reader :entry_store

  # @return [void]
  #[OnStart]
  def open
    @behaviors = Behaviors.create(LedgerSetup, partition: "ledger", signal: BehaviorSignal.new)
  end

  # @param status [String]
  # @return [Hash]
  #[AllOrNothing({"adapter":"ledger"})]
  def settle(status)
    { "open" => entry_store.find_by_status(status), "total" => entry_store.count_by_status(status) }
  end
end
