
struct AppleCryptoKey {
    static let kinds = ["x25519", "ed25519", "encrypt", "derive"]
    let kind: String
    let bytes: Data

    static func stored(_ data: Data) -> AppleCryptoKey? {
        guard let first = data.first, Int(first) < kinds.count, data.count == 33 else { return nil }
        return AppleCryptoKey(kind: kinds[Int(first)], bytes: Data(data.dropFirst()))
    }

    var record: Data {
        Data([UInt8(Self.kinds.firstIndex(of: kind) ?? 0)]) + bytes
    }

    func of(_ wanted: String) throws -> AppleCryptoKey {
        guard kind == wanted else { throw AppleCryptoRefusal.invalid }
        return self
    }

    func publicKey() throws -> Data {
        switch kind {
        case "x25519": return try Curve25519.KeyAgreement.PrivateKey(rawRepresentation: bytes).publicKey.rawRepresentation
        case "ed25519": return try Curve25519.Signing.PrivateKey(rawRepresentation: bytes).publicKey.rawRepresentation
        default: throw AppleCryptoRefusal.invalid
        }
    }
}
