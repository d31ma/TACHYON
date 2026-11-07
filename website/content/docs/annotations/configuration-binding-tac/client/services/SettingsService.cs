using Models;

[Service]
public class SettingsService {
    public string title = "Catalog";
    public long pageSize = 20;

    public SettingsService() {
        var published = BehaviorJSON.Parse("{\"tac\":{\"catalog\":{\"title\":\"Catalog\",\"page\":{\"size\":20},\"cache\":{\"enabled\":true}}}}");
        var catalog = Behaviors.properties(typeof(CatalogSetup), published);
        var values = Behaviors.values(typeof(PublicValues), published);
        title = values.Get("title").Text;
        pageSize = catalog.Get("pageSize").Integer;
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(CatalogSetup), "public-catalog", signal, published, profiles: new[] { "public" });
    }
}
