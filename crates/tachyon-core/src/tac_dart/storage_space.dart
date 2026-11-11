final class StorageSpace {
  const StorageSpace();

  static const _api = 'storageSpace';

  void estimate() => TacWebHost.call(_api, 'estimate');

  void persist() => TacWebHost.call(_api, 'persist');
}
