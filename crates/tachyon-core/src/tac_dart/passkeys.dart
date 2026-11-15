final class Passkeys {
  const Passkeys();

  static const _api = 'passkeys';

  bool supported() => TacWebHost.call(_api, 'supported') as bool;

  void check() => TacWebHost.call(_api, 'check');

  void create(String optionsJson, String prfKey) =>
      TacWebHost.call(_api, 'create', [optionsJson, prfKey]);

  void get(String optionsJson, String prfKey) =>
      TacWebHost.call(_api, 'get', [optionsJson, prfKey]);

  void offer(String optionsJson, String prfKey) =>
      TacWebHost.call(_api, 'offer', [optionsJson, prfKey]);

  void cancel() => TacWebHost.call(_api, 'cancel');
}
