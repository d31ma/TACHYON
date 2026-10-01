import models.PlatformChanged;

@Service
@Lifetime({"value":"prototype"})
class ChoiceService {
    @Needs
    private final PreferenceStrings preferences;

    @Needs
    private final ApplicationEventPublisher events;

    boolean showLate = false;

    String choose(String platform) {
        preferences.write("tachyon.platform", platform);
        events.publishEvent(new PlatformChanged(platform), true);
        return platform;
    }

    String read(String key) {
        return preferences.read(key);
    }

    boolean write(String key, String value) {
        return preferences.write(key, value);
    }
}
