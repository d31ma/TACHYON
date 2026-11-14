final class Locks {
  const Locks();

  static const _api = 'locks';

  void request(String name) => TacWebHost.call(_api, 'request', [name]);

  void release(String name) => TacWebHost.call(_api, 'release', [name]);
}
