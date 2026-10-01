import Foundation

struct NativeUiRuntimeContract: Decodable, Equatable {
    let applicationId: String
    let target: String
    let nativeSha256: String

    func validate() throws {
        try Self.application(applicationId)
        try NativeUiRuntimeFailure.require(target == "macos", "Unsupported native UI platform")
        try NativeUiRuntimeJson.digest(nativeSha256)
    }

    func canonical() -> String {
        "{\"applicationId\":" + NativeUiRuntimeJson.quote(applicationId)
            + ",\"target\":" + NativeUiRuntimeJson.quote(target)
            + ",\"nativeSha256\":" + NativeUiRuntimeJson.quote(nativeSha256) + "}"
    }

    private static func application(_ value: String) throws {
        try boundedApplication(value)
        try NativeUiRuntimeFailure.require(value.utf8.allSatisfy(applicationByte), "Invalid application identity")
    }

    private static func boundedApplication(_ value: String) throws {
        try NativeUiRuntimeFailure.require((1...256).contains(value.utf8.count), "Invalid application identity")
    }

    private static func applicationByte(_ byte: UInt8) -> Bool {
        (48...57).contains(byte) || (65...90).contains(byte)
            || (97...122).contains(byte) || [45, 46, 95].contains(byte)
    }
}
