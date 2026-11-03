using Models;

[RequestRoute("/configuration")]
[Public]
class ConfigurationController {
    [GetRoute]
    public Configuration GetConfiguration() {
        var application = BehaviorJSON.Parse(System.Environment.GetEnvironmentVariable("YON_APPLICATION_JSON")!);
        var settings = Behaviors.properties(typeof(CatalogSettings), application);
        var endpoint = Behaviors.values(typeof(Endpoint), application);
        var pageSize = settings.Get("pageSize").Integer;
        var port = endpoint.Get("port").Integer;
        return new Configuration((int) pageSize, (int) port);
    }
}
