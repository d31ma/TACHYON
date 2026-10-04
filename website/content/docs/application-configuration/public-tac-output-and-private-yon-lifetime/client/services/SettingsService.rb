#[Service]
class SettingsService
  # @return [String]
  attr_reader :api

  def initialize
    @application_client = ApplicationClient.new
    @api = "/api"
  end

  # @return [void]
  #[OnStart]
  def start
    application = @application_client.load
    @api = Behaviors.values(PublicSettings, application)["api"]
  end
end
