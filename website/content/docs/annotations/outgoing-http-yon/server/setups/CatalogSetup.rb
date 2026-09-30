#[Setup]
class CatalogSetup
  # @param execution [Object]
  # @return [Hash]
  def self.configure(execution)
    { http: { catalog: { exchange: method(:exchange) } } }
  end

  # @param request [Hash]
  # @param execution [Object]
  # @return [Hash]
  def self.exchange(request, execution)
    { "items" => ["anvil"] }
  end
end
