final class Crypto {
    private static final String API = "crypto";

    boolean supported() {
        return TacWebHost.flag(API, "supported");
    }

    String check() {
        return request("check");
    }

    String makeKeyPair(String key, String algorithm, boolean keep) {
        return request("makeKeyPair", key, algorithm, keep);
    }

    String publicKey(String key) {
        return request("publicKey", key);
    }

    String sign(String key, String data) {
        return request("sign", key, data);
    }

    String verify(String publicKey, String data, String signature) {
        return request("verify", publicKey, data, signature);
    }

    String agree(String key, String peerPublicKey, String salt, String info, String newKey, String use, boolean keep) {
        return request("agree", key, peerPublicKey, salt, info, newKey, use, keep);
    }

    String deriveKey(String fromKey, String salt, String info, String newKey, String use, boolean keep) {
        return request("deriveKey", fromKey, salt, info, newKey, use, keep);
    }

    String importKey(String newKey, String secret, String use, boolean keep) {
        return request("importKey", newKey, secret, use, keep);
    }

    String fromCode(String newKey, String code, String salt, String use, boolean keep) {
        return request("fromCode", newKey, code, salt, use, keep);
    }

    String encrypt(String key, String data, String aad) {
        return request("encrypt", key, data, aad);
    }

    String decrypt(String key, String sealed, String aad) {
        return request("decrypt", key, sealed, aad);
    }

    String hkdf(String input, String salt, String info, int length) {
        return request("hkdf", input, salt, info, length);
    }

    String hash(String data) {
        return request("hash", data);
    }

    String forget(String key) {
        return request("forget", key);
    }

    String forgetAll() {
        return request("forgetAll");
    }

    private static String request(String call, Object... values) {
        return String.valueOf(TacWebHost.call(API, call, values));
    }
}
