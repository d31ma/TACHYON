final class Notifications {
  const Notifications();

  static const _api = 'notifications';

  String permission() => TacWebHost.call(_api, 'permission') as String;

  void requestPermission() => TacWebHost.call(_api, 'requestPermission');

  void show(String title, String body) =>
      TacWebHost.call(_api, 'show', [title, body]);
}
