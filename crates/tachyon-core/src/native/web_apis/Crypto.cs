public sealed class Crypto
{
    private const string Api = "crypto";

    public bool Supported() => TacWebHost.Call(Api, "supported", new JsonArray()).GetValue<bool>();

    public string Check() => Request("check", new JsonArray());

    public string MakeKeyPair(string key, string algorithm, bool keep) => Request("makeKeyPair", new JsonArray(key, algorithm, keep));

    public string PublicKey(string key) => Request("publicKey", new JsonArray(key));

    public string Sign(string key, string data) => Request("sign", new JsonArray(key, data));

    public string Verify(string publicKey, string data, string signature) => Request("verify", new JsonArray(publicKey, data, signature));

    public string Agree(string key, string peerPublicKey, string salt, string info, string newKey, string use, bool keep) => Request("agree", new JsonArray(key, peerPublicKey, salt, info, newKey, use, keep));

    public string DeriveKey(string fromKey, string salt, string info, string newKey, string use, bool keep) => Request("deriveKey", new JsonArray(fromKey, salt, info, newKey, use, keep));

    public string ImportKey(string newKey, string secret, string use, bool keep) => Request("importKey", new JsonArray(newKey, secret, use, keep));

    public string FromCode(string newKey, string code, string salt, string use, bool keep) => Request("fromCode", new JsonArray(newKey, code, salt, use, keep));

    public string Encrypt(string key, string data, string aad) => Request("encrypt", new JsonArray(key, data, aad));

    public string Decrypt(string key, string sealedBytes, string aad) => Request("decrypt", new JsonArray(key, sealedBytes, aad));

    public string Hkdf(string input, string salt, string info, int length) => Request("hkdf", new JsonArray(input, salt, info, length));

    public string Hash(string data) => Request("hash", new JsonArray(data));

    public string Forget(string key) => Request("forget", new JsonArray(key));

    public string ForgetAll() => Request("forgetAll", new JsonArray());

    private static string Request(string call, JsonArray arguments) => TacWebHost.Call(Api, call, arguments).GetValue<string>();
}
