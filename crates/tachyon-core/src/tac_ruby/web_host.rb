
class TacWebHost
  def self.call(api, call, arguments = [])
    answer = ask({ "api" => api, "call" => call, "args" => arguments })
    value_of(answer)
  end

  def self.ask(request)
    return BrowserFakes.reply(request) unless TachyonWeb.respond_to?(:host_emit)
    TacStorage.ask(request)
  end

  def self.value_of(answer)
    raise answer["error"] if answer["error"].is_a?(String)
    answer["value"]
  end
end
