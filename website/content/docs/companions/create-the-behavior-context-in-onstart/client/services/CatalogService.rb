#[Service]
class CatalogService
  # @return [Integer]
  attr_reader :offset

  def initialize
    @offset = 0
  end

  # @param input [Hash]
  # @return [Integer]
  def page(
    #[PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})]
    input
  )
    input["offset"]
  end

  # @return [void]
  #[OnStart]
  def start
    @behaviors = Behaviors.create(ApplicationSetup, partition: "public-catalog", signal: BehaviorSignal.new)
    @offset = page({ "page" => 1 })
  end
end
