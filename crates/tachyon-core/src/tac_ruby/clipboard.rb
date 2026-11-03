
class Clipboard
  API = "clipboard"

  def write_text(text)
    TacWebHost.call(API, "writeText", [text])
    nil
  end

  def read_text
    TacWebHost.call(API, "readText")
    nil
  end
end
