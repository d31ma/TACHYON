
class Push
  API = "push"

  def supported
    TacWebHost.call(API, "supported") == true
  end

  def subscribe(application_server_key)
    TacWebHost.call(API, "subscribe", [application_server_key])
    nil
  end

  def subscription
    TacWebHost.call(API, "subscription")
    nil
  end

  def unsubscribe
    TacWebHost.call(API, "unsubscribe")
    nil
  end
end
