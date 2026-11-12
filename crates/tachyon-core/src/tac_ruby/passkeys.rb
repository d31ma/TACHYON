
class Passkeys
  API = "passkeys"

  def supported
    TacWebHost.call(API, "supported") == true
  end

  def check
    TacWebHost.call(API, "check")
    nil
  end

  def create(options_json)
    TacWebHost.call(API, "create", [options_json])
    nil
  end

  def get(options_json)
    TacWebHost.call(API, "get", [options_json])
    nil
  end

  def offer(options_json)
    TacWebHost.call(API, "offer", [options_json])
    nil
  end

  def cancel
    TacWebHost.call(API, "cancel")
    nil
  end
end
