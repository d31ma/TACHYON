import models.Configuration

@RequestRoute("/configuration")
@Public
class ConfigurationController {
    @GetRoute
    fun getConfiguration(): Configuration {
        val application = application()
        val settings = Behaviors.properties("CatalogSettings", application)
        val endpoint = Behaviors.values("Endpoint", application)
        val pageSize = (settings["pageSize"] as Number).toInt()
        val port = (endpoint["port"] as Number).toInt()
        return Configuration(pageSize, port)
    }

    private fun application(): Map<String, Any?> =
        (Json.parse(System.getenv("YON_APPLICATION_JSON")) as Map<*, *>).mapKeys { (key, _) -> key as String }
}
