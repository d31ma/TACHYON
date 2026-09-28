
class Notifications
  API = "notifications"

  def permission
    TacWebHost.call(API, "permission").to_s
  end

  def request_permission
    TacWebHost.call(API, "requestPermission")
    nil
  end

  def show(title, body)
    TacWebHost.call(API, "show", [title, body])
    nil
  end
end
