final class Crypto {
    private static let api = "crypto"

    func supported() throws -> Bool {
        try TacWebHost.flag(Self.api, "supported")
    }

    func check() throws -> String {
        try Self.request("check", [])
    }

    func makeKeyPair(key: String, algorithm: String, keep: Bool) throws -> String {
        try Self.request("makeKeyPair", [key, algorithm, keep])
    }

    func publicKey(key: String) throws -> String {
        try Self.request("publicKey", [key])
    }

    func sign(key: String, data: String) throws -> String {
        try Self.request("sign", [key, data])
    }

    func verify(publicKey: String, data: String, signature: String) throws -> String {
        try Self.request("verify", [publicKey, data, signature])
    }

    func agree(key: String, peerPublicKey: String, salt: String, info: String, newKey: String, use: String, keep: Bool) throws -> String {
        try Self.request("agree", [key, peerPublicKey, salt, info, newKey, use, keep])
    }

    func deriveKey(fromKey: String, salt: String, info: String, newKey: String, use: String, keep: Bool) throws -> String {
        try Self.request("deriveKey", [fromKey, salt, info, newKey, use, keep])
    }

    func importKey(newKey: String, secret: String, use: String, keep: Bool) throws -> String {
        try Self.request("importKey", [newKey, secret, use, keep])
    }

    func fromCode(newKey: String, code: String, salt: String, use: String, keep: Bool) throws -> String {
        try Self.request("fromCode", [newKey, code, salt, use, keep])
    }

    func encrypt(key: String, data: String, aad: String) throws -> String {
        try Self.request("encrypt", [key, data, aad])
    }

    func decrypt(key: String, sealed: String, aad: String) throws -> String {
        try Self.request("decrypt", [key, sealed, aad])
    }

    func hkdf(input: String, salt: String, info: String, length: Int) throws -> String {
        try Self.request("hkdf", [input, salt, info, length])
    }

    func hash(data: String) throws -> String {
        try Self.request("hash", [data])
    }

    func forget(key: String) throws -> String {
        try Self.request("forget", [key])
    }

    func forgetAll() throws -> String {
        try Self.request("forgetAll", [])
    }

    private static func request(_ call: String, _ values: [Any?]) throws -> String {
        try TacWebHost.answer(TacWebHost.call(api, call, values), as: String.self)
    }
}
