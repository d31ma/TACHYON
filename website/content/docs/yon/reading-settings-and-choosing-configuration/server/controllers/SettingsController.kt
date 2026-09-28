import models.Settings

@RequestRoute("/settings")
@Public
class SettingsController {
    @GetRoute
    fun getSettings(): Settings {
        val application = application()
        val setup = "CatalogSetup"
        val catalog = Behaviors.properties(setup, application)
        val endpoint = Behaviors.values("Endpoint", application)
        val profile = endpoint["profile"] as String
        Behaviors.create(setup, Behaviors.Options(
            "public-catalog", Behaviors.Signal(), profiles = listOf(profile), properties = application
        )).close()
        return Settings(profile, (catalog["pageSize"] as Number).toInt())
    }

    private fun application(): Map<String, Any?> =
        (Json.parse(System.getenv("YON_APPLICATION_JSON")) as Map<*, *>).mapKeys { (key, _) -> key as String }
}
