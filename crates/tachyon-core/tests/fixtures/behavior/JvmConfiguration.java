class ConfigurationChecks {
    static void check(boolean valid) {
        if (!valid) {
            throw new AssertionError("configuration contract");
        }
    }
    static void fails(String code, Runnable call) {
        try {
            call.run();
            throw new AssertionError("expected " + code);
        } catch (Behaviors.Failure failure) {
            check(failure.getMessage().equals(code));
        }
    }
    static void run() {
        checkSettingsDefaults();
        java.util.Map<String, String> values = new java.util.HashMap<>(java.util.Map.of("YON_APP_TITLE", "Products", "YON_APP_COUNT", "12", "YON_APP_ENABLED", "true", "YON_APP_RATIO", "2.5e0"));
        java.util.Map<String, Object> bound = Behaviors.properties("Settings", values);
        check(Behaviors.properties("Settings", java.util.Map.of("yon", java.util.Map.of("app", java.util.Map.of("title", "Products", "count", 12)))).get("count").equals(12L));
        checkApplicationValues();
        checkApplicationValueLimits();
        checkManifestMembersAreIgnored();
        values.put("YON_APP_TITLE", "changed");
        check(bound.get("title").equals("Products") && bound.get("count").equals(12L) && bound.get("enabled").equals(true) && bound.get("ratio").equals(2.5));
        checkInvalidSettings();
        checkInvalidOptions();
        checkProfileSelection();
        checkSharedSetup();
        System.out.println("Java configuration binding and selection checks passed");
    }
    static void checkSettingsDefaults() {
        java.util.Map<String, Object> defaults = Behaviors.properties("Settings", java.util.Map.of("YON_APP_TITLE", "Products"));
        check(defaults.get("title").equals("Products") && defaults.get("count").equals(2L) && defaults.get("enabled").equals(false) && defaults.get("ratio").equals(1.5));
    }
    static java.util.Map<String, Object> redisValues(String url) {
        return Behaviors.values("ApplicationValues", java.util.Map.of("yon", java.util.Map.of("redis", java.util.Map.of("url", url))));
    }
    static void checkApplicationValues() {
        java.util.Map<String, Object> application = Behaviors.values("ApplicationValues", java.util.Map.of("yon", java.util.Map.of("server", java.util.Map.of("host", "127.0.0.1", "port", 9000), "app", java.util.Map.of("count", 12))));
        check(application.get("port").equals(9000L) && application.get("count").equals(12L) && application.get("redis").equals("redis://127.0.0.1:6379"));
    }
    static void checkApplicationValueLimits() {
        for (String text : new String[]{"x".repeat(4096), "é".repeat(2048)}) {
            check(redisValues(text).get("redis").equals(text));
        }
        for (String text : new String[]{"x".repeat(4097), "é".repeat(2049)}) {
            fails("configuration-properties", () -> redisValues(text));
        }
        fails("configuration-properties", () -> Behaviors.values("ApplicationValues", java.util.Map.of("yon", java.util.Map.of("datasource", java.util.Map.of("password", "x".repeat(4097))))));
        fails("configuration-properties", () -> Behaviors.values("ApplicationValues", java.util.Map.of("server", java.util.Map.of("port", "9000"))));
        fails("configuration-properties", () -> Behaviors.values("ApplicationValues", java.util.Map.of("secrets", java.util.Map.of())));
    }
    static void checkManifestMembersAreIgnored() {
        java.util.Map<String, Object> manifest = java.util.Map.of("name", "Catalog", "icons", java.util.List.of(java.util.Map.of("src", "/icon.svg")), "yon", java.util.Map.of("app", java.util.Map.of("count", 7)));
        check(Behaviors.values("ApplicationValues", manifest).get("count").equals(7L));
    }
    static void failsWithSetting(String name, String value) {
        fails("configuration-properties", () -> Behaviors.properties("Settings", java.util.Map.of("YON_APP_TITLE", "ok", name, value)));
    }
    static void checkInvalidSettings() {
        fails("configuration-properties", () -> Behaviors.properties("Unknown", java.util.Map.of()));
        fails("configuration-properties", () -> Behaviors.properties("Settings", java.util.Map.of()));
        failsWithSetting("YON_APP_TYPO", "secret");
        for (String invalid : new String[]{" 1", "1 ", "+1", "01", "1.0", "1e0", "9007199254740992", "0", "101"}) {
            failsWithSetting("YON_APP_COUNT", invalid);
        }
        for (String invalid : new String[]{"True", "1", " true"}) {
            failsWithSetting("YON_APP_ENABLED", invalid);
        }
        for (String invalid : new String[]{"NaN", "Infinity", "+1", "01", "1e999", " 2", "11"}) {
            failsWithSetting("YON_APP_RATIO", invalid);
        }
        fails("configuration-properties", () -> Behaviors.properties("Settings", java.util.Map.of("YON_APP_TITLE", "x".repeat(4097))));
    }
    static void checkInvalidOptions() {
        fails("configuration", () -> new Behaviors.Options("tenant", new Behaviors.Signal(), 10000, java.util.List.of(), java.util.Map.of("TAC_APP_TITLE", "wrong framework")));
        fails("configuration", () -> new Behaviors.Options("tenant", new Behaviors.Signal(), 10000, java.util.List.of("test", "test"), java.util.Map.of()));
        fails("configuration", () -> new Behaviors.Options("tenant", new Behaviors.Signal(), 10000, java.util.List.of("bad-profile"), java.util.Map.of()));
        java.util.Map<String, String> large = new java.util.HashMap<>();
        for (int index = 0; index < 20; index++) {
            large.put("YON_KEY_" + index, "x".repeat(4096));
        }
        fails("configuration", () -> new Behaviors.Options("tenant", new Behaviors.Signal(), 10000, java.util.List.of(), large));
    }
    static void checkProfileSelection() {
        int configuredBefore = RuntimeAdapters.configured;
        fails("inactive-configuration", () -> Behaviors.create("SelectedAdapters", new Behaviors.Options("tenant", new Behaviors.Signal())));
        fails("inactive-configuration", () -> Behaviors.create("SelectedAdapters", new Behaviors.Options("tenant", new Behaviors.Signal(), 10000, java.util.List.of("test"), java.util.Map.of("YON_ENABLED", "false"))));
        check(RuntimeAdapters.configured == configuredBefore);
        java.util.List<String> profiles = new java.util.ArrayList<>(java.util.List.of("development"));
        java.util.Map<String, String> supplied = new java.util.HashMap<>(java.util.Map.of("YON_ENABLED", "true"));
        Behaviors.Options options = new Behaviors.Options("tenant", new Behaviors.Signal(), 10000, profiles, supplied);
        profiles.clear();
        supplied.put("YON_ENABLED", "false");
        Behaviors.create("SelectedAdapters", options).close();
        check(RuntimeAdapters.configured == configuredBefore + 1);
    }
    static Behaviors.Context createShared(String partition) {
        return Behaviors.create("SelectedAdapters", new Behaviors.Options(partition, new Behaviors.Signal(), 10000, java.util.List.of("development"), java.util.Map.of("YON_ENABLED", "true")));
    }
    static void checkSharedSetup() {
        int configuredBefore = RuntimeAdapters.configured;
        Behaviors.Context writer = createShared("writer");
        Behaviors.Context reader = createShared("reader");
        check(RuntimeAdapters.configured == configuredBefore + 1);
        check(writer.configuration == reader.configuration);
        writer.close();
        writer.close();
        Behaviors.Context later = createShared("later");
        check(RuntimeAdapters.configured == configuredBefore + 1);
        reader.close();
        later.close();
        createShared("fresh").close();
        check(RuntimeAdapters.configured == configuredBefore + 2);
    }
}
