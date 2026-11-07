#[Setup]
class CatalogSetup
  # @param execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(execution)
    {
      entities: { publicCatalog: { execute: method(:execute) } },
      authorizeEntity: method(:product_entity?)
    }
  end

  # @param command [Hash]
  # @param execution [BehaviorExecution]
  # @return [Object]
  def self.execute(command, execution)
    command["query"]["kind"] == "count" ? 0 : []
  end

  # @param command [Hash]
  # @param execution [BehaviorExecution]
  # @return [Boolean]
  def self.product_entity?(command, execution)
    command["entity"] == "ProductEntity"
  end
end
