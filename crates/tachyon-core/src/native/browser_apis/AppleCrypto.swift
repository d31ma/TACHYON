
final class AppleCrypto {
    static let version: UInt8 = 1
    static let keyBytes = 32
    static let sealedExtraBytes = 29
    static let rounds: UInt32 = 600000
    static let algorithms = ["x25519", "ed25519"]
    static let uses = ["encrypt", "derive"]
    static let counts: [String: Int] = [
        "browserApi.crypto.check": 0, "browserApi.crypto.makeKeyPair": 3, "browserApi.crypto.publicKey": 1,
        "browserApi.crypto.sign": 2, "browserApi.crypto.verify": 3, "browserApi.crypto.agree": 7,
        "browserApi.crypto.deriveKey": 6, "browserApi.crypto.importKey": 4, "browserApi.crypto.fromCode": 5,
        "browserApi.crypto.encrypt": 3, "browserApi.crypto.decrypt": 3, "browserApi.crypto.hkdf": 4,
        "browserApi.crypto.hash": 1, "browserApi.crypto.forget": 1, "browserApi.crypto.forgetAll": 0,
    ]
    private let store = AppleKeyStore()
    private let queue = DispatchQueue(label: "dev.tachyon.crypto")

    static func knows(_ call: String) -> Bool {
        counts[call] != nil
    }

    func call(_ call: String, _ payload: String, _ reply: @escaping (String) -> Void) {
        queue.async {
            let answer = self.answer(call, payload)
            DispatchQueue.main.async { reply(answer) }
        }
    }

    private func answer(_ call: String, _ payload: String) -> String {
        do {
            let values = try AppleCryptoValues.parsed(payload, Self.counts[call] ?? 0)
            return AppleBrowserApis.success(try run(call, values))
        } catch let refusal as AppleCryptoRefusal {
            return AppleBrowserApis.success(["reason": refusal.rawValue])
        } catch {
            return AppleBrowserApis.success(["reason": AppleCryptoRefusal.failed.rawValue])
        }
    }

    private func run(_ call: String, _ values: AppleCryptoValues) throws -> [String: Any] {
        switch call {
        case "browserApi.crypto.check": return check()
        case "browserApi.crypto.makeKeyPair": return try makeKeyPair(values)
        case "browserApi.crypto.publicKey": return try publicKey(values)
        case "browserApi.crypto.sign": return try sign(values)
        case "browserApi.crypto.verify": return try verify(values)
        case "browserApi.crypto.agree": return try agree(values)
        case "browserApi.crypto.deriveKey": return try deriveKey(values)
        case "browserApi.crypto.importKey": return try importKey(values)
        case "browserApi.crypto.fromCode": return try fromCode(values)
        case "browserApi.crypto.encrypt": return try encrypt(values)
        case "browserApi.crypto.decrypt": return try decrypt(values)
        case "browserApi.crypto.hkdf": return try hkdf(values)
        case "browserApi.crypto.hash": return try hash(values)
        case "browserApi.crypto.forget": return try forget(values)
        default: return try forgetAll()
        }
    }

    private func check() -> [String: Any] {
        ["x25519": true, "ed25519": true, "aesGcm": true, "hkdf": true, "pbkdf2": true, "keyStore": store.kind()]
    }

    private func makeKeyPair(_ values: AppleCryptoValues) throws -> [String: Any] {
        let name = try values.name(0)
        let algorithm = try values.choice(1, Self.algorithms)
        let keep = try values.flag(2)
        try store.ensureFree(name)
        let key = AppleCryptoKey(kind: algorithm, bytes: Self.newPrivateKey(algorithm))
        try store.save(name, key, keep)
        return Self.made(name, try key.publicKey())
    }

    private func publicKey(_ values: AppleCryptoValues) throws -> [String: Any] {
        let name = try values.name(0)
        return Self.made(name, try store.found(name).publicKey())
    }

    private func sign(_ values: AppleCryptoValues) throws -> [String: Any] {
        let name = try values.name(0)
        let data = try values.bytes(1)
        let key = try store.found(name).of("ed25519")
        let signature = try Curve25519.Signing.PrivateKey(rawRepresentation: key.bytes).signature(for: data)
        return ["signature": AppleCryptoValues.encoded(signature)]
    }

    private func verify(_ values: AppleCryptoValues) throws -> [String: Any] {
        let publicKey = try values.sized(0, Self.keyBytes)
        let data = try values.bytes(1)
        let signature = try values.bytes(2)
        guard let signer = try? Curve25519.Signing.PublicKey(rawRepresentation: publicKey) else {
            throw AppleCryptoRefusal.invalid
        }
        return ["valid": signer.isValidSignature(signature, for: data)]
    }

    private func agree(_ values: AppleCryptoValues) throws -> [String: Any] {
        let name = try values.name(0)
        let peer = try values.sized(1, Self.keyBytes)
        let salt = try values.bytes(2)
        let info = try values.bytes(3)
        let newName = try values.name(4)
        let use = try values.choice(5, Self.uses)
        let keep = try values.flag(6)
        let key = try store.found(name).of("x25519")
        try store.ensureFree(newName)
        let shared = try Self.shared(key, peer)
        let derived = shared.hkdfDerivedSymmetricKey(using: SHA256.self, salt: salt, sharedInfo: info, outputByteCount: Self.keyBytes)
        return try saved(newName, AppleCryptoKey(kind: use, bytes: Self.raw(derived)), keep)
    }

    private func deriveKey(_ values: AppleCryptoValues) throws -> [String: Any] {
        let name = try values.name(0)
        let salt = try values.bytes(1)
        let info = try values.bytes(2)
        let newName = try values.name(3)
        let use = try values.choice(4, Self.uses)
        let keep = try values.flag(5)
        let key = try store.found(name).of("derive")
        try store.ensureFree(newName)
        let derived = Self.expanded(key.bytes, salt, info, Self.keyBytes)
        return try saved(newName, AppleCryptoKey(kind: use, bytes: derived), keep)
    }

    private func importKey(_ values: AppleCryptoValues) throws -> [String: Any] {
        let newName = try values.name(0)
        let secret = try values.sized(1, Self.keyBytes)
        let use = try values.choice(2, Self.uses)
        let keep = try values.flag(3)
        try store.ensureFree(newName)
        return try saved(newName, AppleCryptoKey(kind: use, bytes: secret), keep)
    }

    private func fromCode(_ values: AppleCryptoValues) throws -> [String: Any] {
        let newName = try values.name(0)
        let code = try values.code(1)
        let salt = try values.salt(2)
        let use = try values.choice(3, Self.uses)
        let keep = try values.flag(4)
        try store.ensureFree(newName)
        return try saved(newName, AppleCryptoKey(kind: use, bytes: try Self.stretched(code, salt)), keep)
    }

    private func encrypt(_ values: AppleCryptoValues) throws -> [String: Any] {
        let name = try values.name(0)
        let data = try values.bytes(1)
        let aad = try values.bytes(2)
        let key = try store.found(name).of("encrypt")
        let box = try AES.GCM.seal(data, using: SymmetricKey(data: key.bytes), nonce: AES.GCM.Nonce(), authenticating: aad)
        guard let combined = box.combined else { throw AppleCryptoRefusal.failed }
        return ["sealed": AppleCryptoValues.encoded(Data([Self.version]) + combined)]
    }

    private func decrypt(_ values: AppleCryptoValues) throws -> [String: Any] {
        let name = try values.name(0)
        let sealed = try values.bytes(1, most: AppleCryptoValues.mostBytes + Self.sealedExtraBytes)
        let aad = try values.bytes(2)
        let key = try store.found(name).of("encrypt")
        let data = try Self.opened(sealed, SymmetricKey(data: key.bytes), aad)
        return ["data": AppleCryptoValues.encoded(data)]
    }

    private func hkdf(_ values: AppleCryptoValues) throws -> [String: Any] {
        let input = try values.bytes(0)
        let salt = try values.bytes(1)
        let info = try values.bytes(2)
        let length = try values.length(3)
        return ["bytes": AppleCryptoValues.encoded(Self.expanded(input, salt, info, length))]
    }

    private func hash(_ values: AppleCryptoValues) throws -> [String: Any] {
        let digest = SHA256.hash(data: try values.bytes(0))
        return ["digest": AppleCryptoValues.encoded(Data(digest))]
    }

    private func forget(_ values: AppleCryptoValues) throws -> [String: Any] {
        try store.forget(try values.name(0))
        return [:]
    }

    private func forgetAll() throws -> [String: Any] {
        try store.forgetAll()
        return [:]
    }

    private func saved(_ name: String, _ key: AppleCryptoKey, _ keep: Bool) throws -> [String: Any] {
        try store.save(name, key, keep)
        return Self.made(name, Data())
    }

    private static func made(_ name: String, _ publicKey: Data) -> [String: Any] {
        ["key": name, "publicKey": AppleCryptoValues.encoded(publicKey)]
    }

    private static func newPrivateKey(_ algorithm: String) -> Data {
        guard algorithm == "x25519" else { return Curve25519.Signing.PrivateKey().rawRepresentation }
        return Curve25519.KeyAgreement.PrivateKey().rawRepresentation
    }

    private static func shared(_ key: AppleCryptoKey, _ peer: Data) throws -> SharedSecret {
        guard let mine = try? Curve25519.KeyAgreement.PrivateKey(rawRepresentation: key.bytes),
              let theirs = try? Curve25519.KeyAgreement.PublicKey(rawRepresentation: peer),
              let shared = try? mine.sharedSecretFromKeyAgreement(with: theirs),
              shared.withUnsafeBytes({ bytes in bytes.contains { $0 != 0 } }) else { throw AppleCryptoRefusal.invalid }
        return shared
    }

    private static func expanded(_ input: Data, _ salt: Data, _ info: Data, _ length: Int) -> Data {
        raw(HKDF<SHA256>.deriveKey(inputKeyMaterial: SymmetricKey(data: input), salt: salt, info: info, outputByteCount: length))
    }

    private static func raw(_ key: SymmetricKey) -> Data {
        key.withUnsafeBytes { Data($0) }
    }

    private static func opened(_ sealed: Data, _ key: SymmetricKey, _ aad: Data) throws -> Data {
        guard sealed.count >= sealedExtraBytes, sealed.first == version,
              let box = try? AES.GCM.SealedBox(combined: Data(sealed.dropFirst())),
              let data = try? AES.GCM.open(box, using: key, authenticating: aad) else { throw AppleCryptoRefusal.rejected }
        return data
    }

    private static func stretched(_ code: Data, _ salt: Data) throws -> Data {
        var output = Data(count: keyBytes)
        let status = output.withUnsafeMutableBytes { out in
            code.withUnsafeBytes { password in
                salt.withUnsafeBytes { saltBytes in
                    CCKeyDerivationPBKDF(
                        CCPBKDFAlgorithm(kCCPBKDF2),
                        password.baseAddress?.assumingMemoryBound(to: CChar.self), code.count,
                        saltBytes.baseAddress?.assumingMemoryBound(to: UInt8.self), salt.count,
                        CCPseudoRandomAlgorithm(kCCPRFHmacAlgSHA256), rounds,
                        out.baseAddress?.assumingMemoryBound(to: UInt8.self), keyBytes
                    )
                }
            }
        }
        guard status == kCCSuccess else { throw AppleCryptoRefusal.failed }
        return output
    }
}
