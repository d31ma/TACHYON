#[Setup]
class CatalogSetup
  ROWS = [{ "id" => 1, "status" => "active" }, { "id" => 2, "status" => "retired" }].freeze

  # @param execution [Object]
  # @return [Hash]
  def self.configure(execution)
    { entities: { catalog: { execute: method(:find_rows) } }, authorizeEntity: method(:product_entity?) }
  end

  # @param command [Hash]
  # @param execution [Object]
  # @return [Object]
  def self.find_rows(command, execution)
    return [] unless command["operation"] == "query"
    query = command["query"]
    matching_rows = ROWS.select { |row| matches?(row, query["where"][0]) }
    return matching_rows.length if query["kind"] == "count"
    matching_rows.first(query.dig("page", "size") || matching_rows.length)
  end

  # @param row [Hash]
  # @param condition [Hash]
  # @return [Boolean]
  def self.matches?(row, condition)
    is_equal = row[condition["field"]] == condition["value"]
    condition["operator"] == "ne" ? !is_equal : is_equal
  end

  # @param command [Hash]
  # @param execution [Object]
  # @return [Boolean]
  def self.product_entity?(command, execution)
    command["entity"] == "ProductEntity"
  end
end
