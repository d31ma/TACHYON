#[Helper]
class LedgerAdapterHelper
  TRANSACTION = "transaction"
  ACTION = "action"
  ROWS = [{ "id" => 1, "status" => "open" }, { "id" => 2, "status" => "settled" }].freeze

  @commands = []
  @sequence = 0

  class << self
    # @return [Array<String>]
    attr_reader :commands
  end

  # @param command [Hash]
  # @param execution [Object]
  # @return [Object]
  def self.execute(command, execution)
    return transaction(command) if command["operation"] == TRANSACTION
    query = command["query"]
    @commands << "#{query["kind"]}@#{command.fetch(TRANSACTION, "none")}"
    status = query["where"][0]["value"]
    matching_rows = ROWS.select { |row| row["status"] == status }
    query["kind"] == "count" ? matching_rows.length : matching_rows
  end

  # @param command [Hash]
  # @return [Hash]
  def self.transaction(command)
    identifier = command[TRANSACTION]
    @commands << (identifier ? "#{command[ACTION]}:#{identifier}" : command[ACTION])
    return { "ok" => true } unless command[ACTION] == "begin"
    @sequence += 1
    { TRANSACTION => "t#{@sequence}" }
  end
end
