@Service
class StatusService {
  bool online = true;
  String scheme = 'light';

  @OnEvent({"replay":true})
  void connectivityChanged(ConnectivityChanged changed) {
    online = changed.online;
  }

  @OnEvent({"replay":true})
  void preferencesChanged(PreferencesChanged changed) {
    scheme = changed.scheme;
  }
}
