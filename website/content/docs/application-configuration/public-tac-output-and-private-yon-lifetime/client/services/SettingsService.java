import java.util.Map;

@Service
class SettingsService {
    @Needs
    private final ApplicationClient applicationClient;
    String api = "";

    void load() {
        Map<String, Object> application = (Map<String, Object>) applicationClient.load();
        api = (String) Behaviors.values("PublicSettings", application).get("api");
    }
}
