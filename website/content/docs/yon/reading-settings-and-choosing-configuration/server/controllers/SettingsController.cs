using Models;

[RequestRoute("/settings")]
[Public]
class SettingsController {
    [GetRoute]
    public Settings GetSettings() {
        var application = BehaviorJSON.Parse(System.Environment.GetEnvironmentVariable("YON_APPLICATION_JSON")!);
        var catalog = Behaviors.properties(typeof(CatalogSetup), application);
        var endpoint = Behaviors.values(typeof(Endpoint), application);
        var profile = endpoint.Get("profile").Text;
        using var signal = new BehaviorSignal();
        var profiles = new[] { profile };
        using var context = Behaviors.create(typeof(CatalogSetup), "public-catalog", signal, application, profiles: profiles);
        return new Settings(profile, (int) catalog.Get("pageSize").Integer);
    }
}
