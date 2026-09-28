
class Connectivity
  def online
    TacWebHost.call("connectivity", "online") == true
  end
end
