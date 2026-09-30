#[Service]
class SettingsService
  # @return [String]
  attr_reader :title

  # @return [Integer]
  attr_reader :page_size

  def initialize
    @title = "Catalog"
    @page_size = 20
  end

  # @return [void]
  #[OnStart]
  def bind
    published = public_manifest
    catalog = Behaviors.properties(CatalogSetup, published)
    values = Behaviors.values(PublicValues, published)
    context = Behaviors.create(CatalogSetup, partition: "public-catalog", signal: BehaviorSignal.new,
      profiles: ["public"], properties: published)
    context.dispose
    @title = values["title"]
    @page_size = catalog["pageSize"]
  end

  private

  # @return [Hash]
  def public_manifest
    catalog = { "title" => "Catalog", "page" => { "size" => 20 }, "cache" => { "enabled" => true } }
    { "tac" => { "catalog" => catalog } }
  end
end
