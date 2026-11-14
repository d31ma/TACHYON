final class Crypto {
  const Crypto();

  static const _api = 'crypto';

  bool supported() => TacWebHost.call(_api, 'supported') as bool;

  String check() => _request('check', []);

  String makeKeyPair(String key, String algorithm, bool keep) =>
      _request('makeKeyPair', [key, algorithm, keep]);

  String publicKey(String key) => _request('publicKey', [key]);

  String sign(String key, String data) => _request('sign', [key, data]);

  String verify(String publicKey, String data, String signature) =>
      _request('verify', [publicKey, data, signature]);

  String agree(String key, String peerPublicKey, String salt, String info,
          String newKey, String use, bool keep) =>
      _request('agree', [key, peerPublicKey, salt, info, newKey, use, keep]);

  String deriveKey(String fromKey, String salt, String info, String newKey,
          String use, bool keep) =>
      _request('deriveKey', [fromKey, salt, info, newKey, use, keep]);

  String importKey(String newKey, String secret, String use, bool keep) =>
      _request('importKey', [newKey, secret, use, keep]);

  String fromCode(
          String newKey, String code, String salt, String use, bool keep) =>
      _request('fromCode', [newKey, code, salt, use, keep]);

  String encrypt(String key, String data, String aad) =>
      _request('encrypt', [key, data, aad]);

  String decrypt(String key, String sealed, String aad) =>
      _request('decrypt', [key, sealed, aad]);

  String hkdf(String input, String salt, String info, int length) =>
      _request('hkdf', [input, salt, info, length]);

  String hash(String data) => _request('hash', [data]);

  String forget(String key) => _request('forget', [key]);

  String forgetAll() => _request('forgetAll', []);

  String _request(String call, List<Object> values) =>
      TacWebHost.call(_api, call, values) as String;
}
