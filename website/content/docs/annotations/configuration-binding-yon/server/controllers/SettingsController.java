import models.Settings;

@RequestRoute("/settings")
@Public
class SettingsController {
    @GetRoute
    Settings getSettings() {
        var application = (java.util.Map<?, ?>) Json.parse(System.getenv("YON_APPLICATION_JSON"));
        String setup = "CatalogSetup";
        var catalog = Behaviors.properties(setup, application);
        var endpoint = Behaviors.values("Endpoint", application);
        var profile = (String) endpoint.get("profile");
        var context = Behaviors.create(setup, new Behaviors.Options(
                "public-catalog", new Behaviors.Signal(), 10000, java.util.List.of(profile), application));
        context.close();
        return new Settings(profile, ((Number) catalog.get("pageSize")).intValue());
    }
}
