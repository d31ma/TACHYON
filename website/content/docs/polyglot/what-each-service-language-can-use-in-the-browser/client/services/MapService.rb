#[Service]
class MapService
  # @return [Float]
  attr_reader :latitude
  # @return [Float]
  attr_reader :longitude

  def initialize
    @latitude = 0.0
    @longitude = 0.0
  end

  # @param value [PlaceFound]
  # @return [void]
  #[OnEvent]
  def show(value)
    @latitude = value.latitude
    @longitude = value.longitude
  end
end
