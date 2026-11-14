class Locks
  API = "locks"

  def request(name)
    TacWebHost.call(API, "request", [name])
    nil
  end

  def release(name)
    TacWebHost.call(API, "release", [name])
    nil
  end
end
