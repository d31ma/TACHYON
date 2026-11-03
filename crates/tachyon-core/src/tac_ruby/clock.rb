
class Clock
  def now_ms
    TacWebHost.call("clock", "now").to_f
  end

  def monotonic_ms
    TacWebHost.call("clock", "monotonic").to_f
  end

  def time_zone
    TacWebHost.call("clock", "timeZone").to_s
  end
end
