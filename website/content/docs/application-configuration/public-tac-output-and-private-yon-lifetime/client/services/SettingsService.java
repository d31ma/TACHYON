import java.util.Map;

@Service
class SettingsService {
    private final ApplicationClient applicationClient = new ApplicationClient();
    String api = "";

    void load() {
        Map<String, Object> application = (Map<String, Object>) applicationClient.load();
        api = (String) Behaviors.values("PublicSettings", application).get("api");
    }
}
