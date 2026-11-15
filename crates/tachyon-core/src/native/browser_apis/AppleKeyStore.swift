
final class AppleKeyStore {
    static let service = "dev.tachyon.crypto.keys"
    static let installedMark = "dev.tachyon.crypto.installed"
    static let probeName = "#store-probe"
    static let noEntitlement = "Crypto: this build has no Keychain entitlement (an ad hoc or unsigned build), so Crypto.check reports keyStore none and keep true answers unsupported. Sign a release with TAC_MACOS_PROVISIONING_PROFILE to keep keys.\n"
    private var memory: [String: AppleCryptoKey] = [:]
    private var cleaned = false
    private var warned = false

    func kind() -> String {
        prepare()
        guard SecItemDelete(base(Self.probeName) as CFDictionary) == errSecMissingEntitlement else { return "os" }
        warn()
        return "none"
    }

    func found(_ name: String) throws -> AppleCryptoKey {
        guard let key = try lookup(name) else { throw AppleCryptoRefusal.unknown }
        return key
    }

    func ensureFree(_ name: String) throws {
        guard try lookup(name) == nil else { throw AppleCryptoRefusal.exists }
    }

    func save(_ name: String, _ key: AppleCryptoKey, _ keep: Bool) throws {
        guard keep else { memory[name] = key; return }
        prepare()
        var item = base(name)
        item[kSecValueData as String] = key.record
        item[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        let status = SecItemAdd(item as CFDictionary, nil)
        if status == errSecMissingEntitlement { warn() }
        try Self.check(status, missing: .unsupported)
    }

    func forget(_ name: String) throws {
        memory[name] = nil
        prepare()
        try Self.check(SecItemDelete(base(name) as CFDictionary), missing: nil)
    }

    func forgetAll() throws {
        memory.removeAll()
        prepare()
        try Self.check(SecItemDelete(base() as CFDictionary), missing: nil)
    }

    private func lookup(_ name: String) throws -> AppleCryptoKey? {
        if let key = memory[name] { return key }
        prepare()
        var query = base(name)
        query[kSecReturnData as String] = true
        var found: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &found)
        try Self.check(status, missing: nil)
        return (found as? Data).flatMap(AppleCryptoKey.stored)
    }

    private func warn() {
        guard !warned else { return }
        warned = true
        FileHandle.standardError.write(Data(Self.noEntitlement.utf8))
    }

    private func prepare() {
        guard !cleaned else { return }
        cleaned = true
        guard !UserDefaults.standard.bool(forKey: Self.installedMark) else { return }
        let status = SecItemDelete(base() as CFDictionary)
        guard [errSecSuccess, errSecItemNotFound].contains(status) else { return }
        UserDefaults.standard.set(true, forKey: Self.installedMark)
    }

    private func base(_ name: String? = nil) -> [String: Any] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: Self.service,
            kSecAttrSynchronizable as String: false,
            kSecUseDataProtectionKeychain as String: true,
        ]
        if let name { query[kSecAttrAccount as String] = name }
        return query
    }

    private static func check(_ status: OSStatus, missing: AppleCryptoRefusal?) throws {
        switch status {
        case errSecSuccess, errSecItemNotFound: return
        case errSecMissingEntitlement: if let missing { throw missing }
        case errSecDuplicateItem: throw AppleCryptoRefusal.exists
        case errSecInteractionNotAllowed: throw AppleCryptoRefusal.locked
        default: throw AppleCryptoRefusal.failed
        }
    }
}
