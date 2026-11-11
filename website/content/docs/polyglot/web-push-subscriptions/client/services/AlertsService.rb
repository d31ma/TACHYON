#[Service]
class AlertsService
  SERVER_KEY = "BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U"

  # @return [String]
  attr_reader :endpoint
  # @return [String]
  attr_reader :problem

  def initialize
    @push = Push.new
    @endpoint = ""
    @problem = ""
  end

  # @return [void]
  def turn_on
    keep_problem { @push.subscribe(SERVER_KEY) }
  end

  # @return [void]
  def turn_off
    keep_problem { @push.unsubscribe }
  end

  # @param subscribed [PushSubscribed]
  # @return [void]
  #[OnEvent]
  def subscribed(subscribed)
    @endpoint = subscribed.endpoint
  end

  # @param unsubscribed [PushUnsubscribed]
  # @return [void]
  #[OnEvent]
  def unsubscribed(unsubscribed)
    @endpoint = ""
  end

  # @param failed [BrowserCallFailed]
  # @return [void]
  #[OnEvent]
  def failed(failed)
    @problem = failed.message
  end

  private

  # @return [void]
  def keep_problem
    yield
  rescue RuntimeError => refusal
    @problem = refusal.message
  end
end
