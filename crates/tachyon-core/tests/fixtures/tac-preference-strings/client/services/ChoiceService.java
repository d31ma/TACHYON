@Service
@Lifetime({"value":"prototype"})
class ChoiceService {
    @Needs
    private final PreferenceStrings preferences;

    String read(String key) {
        return preferences.read(key);
    }

    boolean write(String key, String value) {
        return preferences.write(key, value);
    }
}
