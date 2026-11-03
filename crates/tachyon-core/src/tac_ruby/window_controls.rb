
class WindowControls
  def granted(control)
    TacWebHost.call("windowControls", "granted", [control]) == true
  end

  def minimize
    TacWebHost.call("windowControls", "minimize")
    nil
  end

  def maximize
    TacWebHost.call("windowControls", "maximize")
    nil
  end

  def fullscreen
    TacWebHost.call("windowControls", "fullscreen")
    nil
  end

  def close
    TacWebHost.call("windowControls", "close")
    nil
  end

  def resize(width, height)
    TacWebHost.call("windowControls", "resize", [width, height])
    nil
  end

  def set_title(text)
    TacWebHost.call("windowControls", "setTitle", [text])
    nil
  end
end
