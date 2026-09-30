#[RequestRoute("/settings")]
#[Public]
class SettingsController
  # @return [Settings]
  #[GetRoute]
  def get_settings
    application = JSON.parse(ENV.fetch("YON_APPLICATION_JSON"))
    catalog = Behaviors.properties(CatalogSetup, application)
    profile = Behaviors.values(Endpoint, application)["profile"]
    context = Behaviors.create(CatalogSetup, partition: "public-catalog", signal: BehaviorSignal.new,
      profiles: [profile], properties: application)
    context.dispose
    Settings.new(profile: profile, pageSize: catalog["pageSize"])
  end
end
