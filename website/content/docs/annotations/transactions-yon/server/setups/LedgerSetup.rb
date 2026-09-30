#[Setup]
class LedgerSetup
  # @param execution [Object]
  # @return [Hash]
  def self.configure(execution)
    { entities: { ledger: { execute: LedgerAdapterHelper.method(:execute) } }, authorizeEntity: method(:authorize) }
  end

  # @param command [Hash]
  # @param execution [Object]
  # @return [Boolean]
  def self.authorize(command, execution)
    true
  end
end
