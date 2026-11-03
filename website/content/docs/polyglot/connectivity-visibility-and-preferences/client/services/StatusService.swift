@Service
final class StatusService {
    var online: Bool = false
    var scheme: String = ""

    @OnEvent({"replay":true})
    func connectivityChanged(_ changed: ConnectivityChanged) {
        online = changed.online
    }

    @OnEvent({"replay":true})
    func preferencesChanged(_ changed: PreferencesChanged) {
        scheme = changed.scheme
    }
}
