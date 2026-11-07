@Test(SiteTestSetup.class, {"partition":"tests"})
class PlatformChoiceServiceTest {
    @Fake
    PreferenceStrings preferences;
    @Fake
    ApplicationEventPublisher applicationEventPublisher;

    void testChoosePublishesAndRemembersThePlatform() {
        java.util.Map<String, String> stored = new java.util.HashMap<>();
        java.util.List<PlatformChanged> changed = new java.util.ArrayList<>();
        java.util.List<Boolean> retained = new java.util.ArrayList<>();
        preferences = storage(stored, false);
        applicationEventPublisher = publisher(changed, retained);
        PlatformChoiceService choice = construct(PlatformChoiceService.class);
        choice.choose("windows");
        Assert.isTrue(changed.get(0) instanceof PlatformChanged, "choose announces the PlatformChanged model");
        Assert.equals("windows", changed.get(0).platform(), "choose announces the platform");
        Assert.equals(true, retained.get(0), "retained, so a snippet mounted later reads it");
        Assert.equals("windows", stored.get("tachyon.platform"), "and remembers it under the exact raw key");
        Assert.equals(1, stored.size(), "no prefixed or serialized storage key is added");
    }

    void testDeniedStorageStillPublishesTheRetainedPlatform() {
        java.util.Map<String, String> stored = new java.util.HashMap<>();
        stored.put("tachyon.platform", "macos");
        java.util.List<PlatformChanged> changed = new java.util.ArrayList<>();
        java.util.List<Boolean> retained = new java.util.ArrayList<>();
        preferences = storage(stored, true);
        applicationEventPublisher = publisher(changed, retained);
        construct(PlatformChoiceService.class).choose("windows");
        Assert.equals("macos", stored.get("tachyon.platform"), "denied storage remains unchanged");
        Assert.equals("windows", changed.get(0).platform(), "denial does not suppress the event");
        Assert.equals(true, retained.get(0), "denied selection remains replayable");
    }

    void testConstructedServicesKeepTheirOwnTypedFakes() {
        java.util.Map<String, String> firstStore = new java.util.HashMap<>();
        java.util.List<PlatformChanged> firstEvents = new java.util.ArrayList<>();
        java.util.List<Boolean> firstRetained = new java.util.ArrayList<>();
        preferences = storage(firstStore, false);
        applicationEventPublisher = publisher(firstEvents, firstRetained);
        PlatformChoiceService first = construct(PlatformChoiceService.class);
        java.util.Map<String, String> secondStore = new java.util.HashMap<>();
        java.util.List<PlatformChanged> secondEvents = new java.util.ArrayList<>();
        java.util.List<Boolean> secondRetained = new java.util.ArrayList<>();
        preferences = storage(secondStore, false);
        applicationEventPublisher = publisher(secondEvents, secondRetained);
        PlatformChoiceService second = construct(PlatformChoiceService.class);
        first.choose("macOS");
        second.choose("windows");
        Assert.equals("macOS", firstStore.get("tachyon.platform"), "raw case is not normalized");
        Assert.equals("windows", secondStore.get("tachyon.platform"), "second storage fake is independent");
        Assert.equals("macOS", firstEvents.get(0).platform(), "first publisher fake stays captured");
        Assert.equals("windows", secondEvents.get(0).platform(), "second publisher fake stays captured");
        Assert.equals(java.util.List.of(true), firstRetained, "first retention is preserved");
        Assert.equals(java.util.List.of(true), secondRetained, "second retention is preserved");
    }

    private PreferenceStrings storage(java.util.Map<String, String> stored, boolean denied) {
        return new PreferenceStrings() {
            public String read(String key) { return stored.get(key); }
            public boolean write(String key, String value) {
                if (denied) return false;
                stored.put(key, value);
                return true;
            }
        };
    }

    private ApplicationEventPublisher publisher(java.util.List<PlatformChanged> changed, java.util.List<Boolean> retained) {
        return new ApplicationEventPublisher() {
            @Override
            public <T> T publishEvent(T value, boolean retain) {
                changed.add((PlatformChanged) value);
                retained.add(retain);
                return value;
            }
        };
    }
}
