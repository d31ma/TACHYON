#[Service]
class ShowcaseService
  TAC_SUMMARY = "One component runs on every target. The view and the stylesheet are the same everywhere. Pick a platform to see every language that can write the service there."
  YON_SUMMARY = "In all eleven languages, a controller uses the same route annotations. Services, stores and other application modules stay ordinary code. The language itself puts them together."

  # @return [String]
  attr_reader :layer

  # @return [void]
  #[OnStart]
  def start
    @layer = "tac"
  end

  # @param value [String]
  # @return [void]
  def pick_layer(value)
    @layer = value
  end

  # @return [String]
  def summary
    summary_for(@layer)
  end

  private

  # @param layer [String]
  # @return [String]
  def summary_for(layer)
    return TAC_SUMMARY if layer == "tac"

    YON_SUMMARY
  end
end
