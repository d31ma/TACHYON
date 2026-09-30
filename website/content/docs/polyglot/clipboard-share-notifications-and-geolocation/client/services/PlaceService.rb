#[Service]
class PlaceService
  # @return [String]
  attr_reader :place
  # @return [String]
  attr_reader :problem

  def initialize
    @geolocation = Geolocation.new
    @clipboard = Clipboard.new
    @place = ""
    @problem = ""
  end

  # @return [void]
  def locate
    keep_problem { @geolocation.current_position }
  end

  # @return [void]
  def copy
    keep_problem { @clipboard.write_text(@place) }
  end

  # @param found [PositionFound]
  # @return [void]
  #[OnEvent]
  def found(found)
    @place = "#{found.latitude}, #{found.longitude}"
  end

  # @param failed [BrowserCallFailed]
  # @return [void]
  #[OnEvent]
  def failed(failed)
    @problem = failed.message
  end

  private

  # @return [void]
  def keep_problem
    yield
  rescue RuntimeError => refusal
    @problem = refusal.message
  end
end
