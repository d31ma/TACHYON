@Service
class ActivityService {
    var state: String = "foreground"

    @OnEvent({"replay":true})
    fun applicationStateChanged(changed: ApplicationStateChanged) {
        state = changed.state
    }
}
