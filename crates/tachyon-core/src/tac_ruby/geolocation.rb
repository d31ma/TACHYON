
class Geolocation
  def current_position
    TacWebHost.call("geolocation", "currentPosition")
    nil
  end
end
