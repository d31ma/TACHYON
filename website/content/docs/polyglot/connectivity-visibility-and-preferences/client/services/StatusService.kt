@Service
class StatusService {
    var online: Boolean = true
    var scheme: String = "light"

    @OnEvent({"replay":true})
    fun connectivityChanged(changed: ConnectivityChanged) {
        online = changed.online
    }

    @OnEvent({"replay":true})
    fun preferencesChanged(changed: PreferencesChanged) {
        scheme = changed.scheme
    }
}
