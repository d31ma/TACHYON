@Service
class SettingsService {
    @Needs
    private val applicationClient: ApplicationClient
    var api: String = ""

    suspend fun load() {
        val application = applicationClient.load() as Map<*, *>
        api = Behaviors.values("PublicSettings", application)["api"] as String
    }
}
