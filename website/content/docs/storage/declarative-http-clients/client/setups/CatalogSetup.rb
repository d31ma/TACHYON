#[Setup]
class CatalogSetup
  # @param _execution [BehaviorExecution]
  # @return [Hash]
  def self.configure(_execution)
    catalog = { "exchange" => ->(_request, _running) { %w[anvil rope] } }
    { "http" => { "catalog" => catalog } }
  end
end
