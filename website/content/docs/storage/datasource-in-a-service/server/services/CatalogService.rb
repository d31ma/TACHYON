#[Service]
class CatalogService
  #[Needs]
  # @return [DataSource]
  attr_reader :data_source

  # @return [Array<Hash>]
  def find
    data_source.query("SELECT 42 AS amount", [])
  end
end
