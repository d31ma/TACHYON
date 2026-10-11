using Models;

[Service]
public class SettingsService {
    [Needs]
    private readonly ApplicationClient _applicationClient;
    public string api = "";

    public async System.Threading.Tasks.Task Load() {
        var application = await _applicationClient.Load();
        api = Behaviors.values(typeof(PublicSettings), application).Get("api").Text;
    }
}
