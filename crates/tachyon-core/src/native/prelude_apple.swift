import Foundation

public enum TacStore {
    nonisolated(unsafe) private static var values: [String: Any] = [:]

    public static func local<Value>(_ key: String, _ fallback: Value) -> Value {
        UserDefaults.standard.object(forKey: "tachyon." + key) as? Value ?? fallback
    }

    public static func setLocal(_ key: String, _ value: Any) {
        UserDefaults.standard.set(value, forKey: "tachyon." + key)
    }

    public static func session<Value>(_ key: String, _ fallback: Value) -> Value {
        values[key] as? Value ?? fallback
    }

    public static func setSession(_ key: String, _ value: Any) {
        values[key] = value
    }
}

func tacParseJSON(_ text: String) -> Any? {
    tachyonSwiftJSONValue(
        try? JSONSerialization.jsonObject(with: Data(text.utf8), options: [.fragmentsAllowed]))
}

func tacModel<T: Decodable>(_ type: T.Type, _ value: Any?) throws -> T {
    try JSONDecoder().decode(T.self, from: Data(tacEncode(value).utf8))
}

func tacModelValue<T: Encodable>(_ value: T) -> Any? {
    guard let data = try? JSONEncoder().encode(value) else { return nil }
    return tacParseJSON(String(decoding: data, as: UTF8.self))
}

func tacValidOwner(_ owner: String) -> Bool { UUID(uuidString: owner) != nil }

func tacFailureAnswer(_ error: Error) -> String { "{\"error\":\"Native companion operation failed.\"}" }

enum TacPlatform {
    static let pageListeners = false
    static let taskHost = false
    static let pageServices = true
}

func tacOnOwnerThread() -> Bool { Thread.isMainThread }

func tacEventFailure() { FileHandle.standardError.write(Data("Tachyon native event delivery failed.\n".utf8)) }

func tacAwait<Value>(_ work: @escaping () async throws -> Value) throws -> Value {
    let awaited = TacAwaited<Value>()
    let finished = DispatchSemaphore(value: 0)
    Task.detached {
        await awaited.finish(work)
        finished.signal()
    }
    TacOwnerThread.wait(for: finished)
    return try awaited.value()
}

func tacBeginTask(_ owner: String, _ work: @escaping ([Any?]) async throws -> Any?, _ arguments: [Any?]) throws -> String {
    "{\"value\":" + tacEncode(try tacAwait { try await work(arguments) }) + "}"
}

func tacCancelTasks(_ owner: String?) {}

func tacCreateNativeOwner<T>(_ owner: String, _ work: () -> T) -> T {
    NativeCapabilities.create(owner, work)
}

func tacDisposeNativeOwner(_ owner: String) {
    NativeCapabilities.dispose(owner)
}
