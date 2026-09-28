
class Preferences
  def scheme
    text("scheme")
  end

  def motion
    text("motion")
  end

  def language
    text("language")
  end

  private

  def text(call)
    TacWebHost.call("preferences", call).to_s
  end
end
