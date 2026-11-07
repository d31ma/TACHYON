
class Share
  def open(title, text, url)
    TacWebHost.call("share", "open", [title, text, url])
    nil
  end
end
