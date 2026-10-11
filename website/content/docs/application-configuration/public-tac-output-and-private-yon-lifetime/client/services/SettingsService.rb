#[Service]
class SettingsService
  #[Needs]
  # @return [ApplicationClient]
  attr_reader :application_client

  # @return [String]
  attr_reader :api

  def initialize
    @api = ""
  end

  # @return [void]
  def load
    application = application_client.load
    @api = Behaviors.values(PublicSettings, application)["api"]
  end
end
