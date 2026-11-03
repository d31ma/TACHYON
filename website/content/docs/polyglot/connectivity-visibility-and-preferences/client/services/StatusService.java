@Service
class StatusService {
    boolean online = false;
    String scheme = "";

    @OnEvent({"replay":true})
    void connectivityChanged(ConnectivityChanged changed) {
        online = changed.online();
    }

    @OnEvent({"replay":true})
    void preferencesChanged(PreferencesChanged changed) {
        scheme = changed.scheme();
    }
}
