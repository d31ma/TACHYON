@Service
final class StatusService {
    private let pageVisibility = PageVisibility()
    var online: Bool = false
    var visible: Bool = false

    @OnEvent({"replay":true})
    func connectivityChanged(_ changed: ConnectivityChanged) {
        online = changed.online
    }

    func check() {
        visible = (try? pageVisibility.visible()) ?? true
    }
}
