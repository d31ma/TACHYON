final class Push {
  const Push();

  static const _api = 'push';

  bool supported() => TacWebHost.call(_api, 'supported') as bool;

  void subscribe(String applicationServerKey) =>
      TacWebHost.call(_api, 'subscribe', [applicationServerKey]);

  void subscription() => TacWebHost.call(_api, 'subscription');

  void unsubscribe() => TacWebHost.call(_api, 'unsubscribe');
}
