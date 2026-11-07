
class SecureRandom
  def bytes(count)
    TacWebHost.call("secureRandom", "bytes", [count])
  end

  def uuid
    TacWebHost.call("secureRandom", "uuid").to_s
  end
end
