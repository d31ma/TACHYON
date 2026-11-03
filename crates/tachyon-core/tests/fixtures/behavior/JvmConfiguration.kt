object ConfigurationChecks {
    private fun fails(code: String, call: () -> Unit) {
        try {
            call()
            error("expected $code")
        } catch (failure: Behaviors.Failure) {
            check(failure.code == code)
        }
    }
    fun run() {
        checkSettingsDefaults()
        val values = mutableMapOf("YON_APP_TITLE" to "Products", "YON_APP_COUNT" to "12", "YON_APP_ENABLED" to "true", "YON_APP_RATIO" to "2.5e0")
        val bound = Behaviors.properties("Settings", values)
        check(Behaviors.properties("Settings", mapOf("yon" to mapOf("app" to mapOf("title" to "Products", "count" to 12))))["count"] == 12L)
        checkApplicationValues()
        checkApplicationValueLimits()
        checkManifestMembersAreIgnored()
        values["YON_APP_TITLE"] = "changed"
        check(bound["title"] == "Products" && bound["count"] == 12L && bound["enabled"] == true && bound["ratio"] == 2.5)
        checkInvalidSettings()
        checkInvalidOptions()
        checkProfileSelection()
        checkSharedSetup()
        println("Kotlin configuration binding and selection checks passed")
    }
    private fun checkSettingsDefaults() {
        val defaults = Behaviors.properties("Settings", mapOf("YON_APP_TITLE" to "Products"))
        check(defaults["title"] == "Products" && defaults["count"] == 2L && defaults["enabled"] == false && defaults["ratio"] == 1.5)
    }
    private fun redisValues(url: String): Map<String, Any?> {
        return Behaviors.values("ApplicationValues", mapOf("yon" to mapOf("redis" to mapOf("url" to url))))
    }
    private fun checkApplicationValues() {
        val application = Behaviors.values("ApplicationValues", mapOf("yon" to mapOf("server" to mapOf("host" to "127.0.0.1", "port" to 9000), "app" to mapOf("count" to 12))))
        check(application["port"] == 9000L && application["count"] == 12L && application["redis"] == "redis://127.0.0.1:6379")
    }
    private fun checkApplicationValueLimits() {
        for (text in listOf("x".repeat(4096), "é".repeat(2048))) {
            check(redisValues(text)["redis"] == text)
        }
        for (text in listOf("x".repeat(4097), "é".repeat(2049))) {
            fails("configuration-properties") { redisValues(text) }
        }
        fails("configuration-properties") { Behaviors.values("ApplicationValues", mapOf("yon" to mapOf("datasource" to mapOf("password" to "x".repeat(4097))))) }
        fails("configuration-properties") { Behaviors.values("ApplicationValues", mapOf("server" to mapOf("port" to "9000"))) }
        fails("configuration-properties") { Behaviors.values("ApplicationValues", mapOf("secrets" to emptyMap<String, String>())) }
    }
    private fun checkManifestMembersAreIgnored() {
        val manifest = mapOf("name" to "Catalog", "icons" to listOf(mapOf("src" to "/icon.svg")), "yon" to mapOf("app" to mapOf("count" to 7)))
        check(Behaviors.values("ApplicationValues", manifest)["count"] == 7L)
    }
    private fun failsWithSetting(name: String, value: String) {
        fails("configuration-properties") { Behaviors.properties("Settings", mapOf("YON_APP_TITLE" to "ok", name to value)) }
    }
    private fun checkInvalidSettings() {
        fails("configuration-properties") { Behaviors.properties("Unknown", emptyMap()) }
        fails("configuration-properties") { Behaviors.properties("Settings", emptyMap()) }
        failsWithSetting("YON_APP_TYPO", "secret")
        for (invalid in listOf(" 1", "1 ", "+1", "01", "1.0", "1e0", "9007199254740992", "0", "101")) {
            failsWithSetting("YON_APP_COUNT", invalid)
        }
        for (invalid in listOf("True", "1", " true")) {
            failsWithSetting("YON_APP_ENABLED", invalid)
        }
        for (invalid in listOf("NaN", "Infinity", "+1", "01", "1e999", " 2", "11")) {
            failsWithSetting("YON_APP_RATIO", invalid)
        }
        fails("configuration-properties") { Behaviors.properties("Settings", mapOf("YON_APP_TITLE" to "x".repeat(4097))) }
    }
    private fun checkInvalidOptions() {
        fails("configuration") { Behaviors.Options("tenant", Behaviors.Signal(), properties = mapOf("TAC_APP_TITLE" to "wrong framework")) }
        fails("configuration") { Behaviors.Options("tenant", Behaviors.Signal(), profiles = listOf("test", "test")) }
        fails("configuration") { Behaviors.Options("tenant", Behaviors.Signal(), profiles = listOf("bad-profile")) }
        val large = mutableMapOf<String, String>()
        for (index in 0 until 20) {
            large["YON_KEY_$index"] = "x".repeat(4096)
        }
        fails("configuration") { Behaviors.Options("tenant", Behaviors.Signal(), properties = large) }
    }
    private fun checkProfileSelection() {
        val configuredBefore = RuntimeAdapters.configured
        fails("inactive-configuration") { Behaviors.create("SelectedAdapters", Behaviors.Options("tenant", Behaviors.Signal())) }
        fails("inactive-configuration") { Behaviors.create("SelectedAdapters", Behaviors.Options("tenant", Behaviors.Signal(), profiles = listOf("test"), properties = mapOf("YON_ENABLED" to "false"))) }
        check(RuntimeAdapters.configured == configuredBefore)
        val profiles = mutableListOf("development")
        val supplied = mutableMapOf("YON_ENABLED" to "true")
        val options = Behaviors.Options("tenant", Behaviors.Signal(), profiles = profiles, properties = supplied)
        profiles.clear()
        supplied["YON_ENABLED"] = "false"
        Behaviors.create("SelectedAdapters", options).close()
        check(RuntimeAdapters.configured == configuredBefore + 1)
    }
    private fun createShared(partition: String): Behaviors.Context =
        Behaviors.create("SelectedAdapters", Behaviors.Options(partition, Behaviors.Signal(), profiles = listOf("development"), properties = mapOf("YON_ENABLED" to "true")))
    private fun checkSharedSetup() {
        val configuredBefore = RuntimeAdapters.configured
        val writer = createShared("writer")
        val reader = createShared("reader")
        check(RuntimeAdapters.configured == configuredBefore + 1)
        check(writer.configuration === reader.configuration)
        writer.close()
        writer.close()
        val later = createShared("later")
        check(RuntimeAdapters.configured == configuredBefore + 1)
        reader.close()
        later.close()
        createShared("fresh").close()
        check(RuntimeAdapters.configured == configuredBefore + 2)
    }
}
