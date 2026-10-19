@Service
class ActivityService {
    String state = "foreground";

    @OnEvent({"replay":true})
    void applicationStateChanged(ApplicationStateChanged changed) {
        state = changed.state();
    }
}
