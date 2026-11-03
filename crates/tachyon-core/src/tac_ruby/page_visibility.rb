
class PageVisibility
  def visible
    TacWebHost.call("pageVisibility", "visible") == true
  end
end
