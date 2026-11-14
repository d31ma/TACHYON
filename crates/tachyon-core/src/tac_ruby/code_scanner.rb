
class CodeScanner
  API = "codeScanner"

  def supported
    TacWebHost.call(API, "supported") == true
  end

  def scan
    TacWebHost.call(API, "scan")
    nil
  end

  def cancel
    TacWebHost.call(API, "cancel")
    nil
  end
end
