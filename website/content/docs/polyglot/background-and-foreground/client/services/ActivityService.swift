@Service
final class ActivityService {
    var state: String = "foreground"

    @OnEvent({"replay":true})
    func applicationStateChanged(_ changed: ApplicationStateChanged) {
        state = changed.state
    }
}
