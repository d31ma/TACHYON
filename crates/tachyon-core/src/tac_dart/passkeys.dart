final class Passkeys {
  const Passkeys();

  static const _api = 'passkeys';

  bool supported() => TacWebHost.call(_api, 'supported') as bool;

  void check() => TacWebHost.call(_api, 'check');

  void create(String optionsJson) =>
      TacWebHost.call(_api, 'create', [optionsJson]);

  void get(String optionsJson) => TacWebHost.call(_api, 'get', [optionsJson]);

  void offer(String optionsJson) =>
      TacWebHost.call(_api, 'offer', [optionsJson]);

  void cancel() => TacWebHost.call(_api, 'cancel');
}
