import models.PlatformChanged;

@Service
class PlatformChoiceService {
    @Needs
    private final PreferenceStrings preferences;
    @Needs
    private final ApplicationEventPublisher applicationEventPublisher;

    void choose(String id) {
        preferences.write("tachyon.platform", id);
        applicationEventPublisher.publishEvent(new PlatformChanged(id), true);
    }
}
