
class Focus
  def move(target)
    TacWebHost.call("focus", "move", [target])
    nil
  end

  def trap(container)
    TacWebHost.call("focus", "trap", [container])
    nil
  end

  def release
    TacWebHost.call("focus", "release")
    nil
  end
end
