import models.Configuration;

@RequestRoute("/configuration")
@Public
class ConfigurationController {
    @GetRoute
    Configuration getConfiguration() {
        var applicationJson = System.getenv("YON_APPLICATION_JSON");
        var application = (java.util.Map<?, ?>) Json.parse(applicationJson);
        var settings = Behaviors.properties("CatalogSettings", application);
        var endpoint = Behaviors.values("Endpoint", application);
        int pageSize = ((Number) settings.get("pageSize")).intValue();
        int port = ((Number) endpoint.get("port")).intValue();
        return new Configuration(pageSize, port);
    }
}
