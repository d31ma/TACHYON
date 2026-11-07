#[RequestRoute("/configuration")]
#[Public]
class ConfigurationController
  # @return [Configuration]
  #[GetRoute]
  def get_configuration
    application = JSON.parse(ENV.fetch("YON_APPLICATION_JSON"))
    settings = Behaviors.properties(CatalogSettings, application)
    endpoint = Behaviors.values(Endpoint, application)
    Configuration.new(pageSize: settings["page_size"], port: endpoint["port"])
  end
end
