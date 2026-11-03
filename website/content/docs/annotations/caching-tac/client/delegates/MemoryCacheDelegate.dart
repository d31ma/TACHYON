@Delegate
final class MemoryCacheDelegate {
  final Map<String, Map<String, Object?>> _entries = {};

  BehaviorCacheAdapter adapter() {
    return BehaviorCacheAdapter(
      get: (key, execution) async => _entries[key] ?? <String, Object?>{'hit': false},
      set: (key, value, policy, expiresAt, execution) async {
        _entries[key] = {'hit': true, 'value': value, 'expiresAt': expiresAt};
      },
      delete: (key, execution) async {
        _entries.remove(key);
      },
    );
  }
}
