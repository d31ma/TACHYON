func tachyonSwiftJSONValue(_ value: Any?) -> Any? {
    switch value {
    case nil, is TacJSONReader.Null: return nil
    case let real as Double: return Int(exactly: real) ?? real
    case let values as [Any]: return values.map(tachyonSwiftJSONValue)
    case let values as [String: Any]: return values.mapValues(tachyonSwiftJSONValue)
    default: return value
    }
}

func tachyonParseJSONRequest(_ raw: String) -> [String: Any]? {
    var reader = TacJSONReader(raw)
    return reader.document() as? [String: Any]
}

func tacParseJSON(_ text: String) -> Any? {
    var reader = TacJSONReader(text)
    return tachyonSwiftJSONValue(reader.document())
}

func tacModel<T: Decodable>(_ type: T.Type, _ value: Any?) throws -> T {
    do { return try TacValueDecoder.model(type, value) } catch { throw TacWrongArgument.value }
}

func tacModelValue<T: Encodable>(_ value: T) -> Any? {
    (try? TacValueEncoder.plain(value)) ?? nil
}

func tacValidOwner(_ owner: String) -> Bool {
    let dashes: Set<Int> = [8, 13, 18, 23]
    let characters = Array(owner.utf8)
    guard characters.count == 36 else { return false }
    return characters.indices.allSatisfy { position in
        dashes.contains(position)
            ? characters[position] == UInt8(ascii: "-")
            : UInt8(ascii: "0")...UInt8(ascii: "9") ~= characters[position]
                || UInt8(ascii: "a")...UInt8(ascii: "f") ~= characters[position]
                || UInt8(ascii: "A")...UInt8(ascii: "F") ~= characters[position]
    }
}

func tacFailureAnswer(_ error: Error) -> String {
    "{\"error\":" + tacEncode(String(describing: error)) + "}"
}

enum TacPlatform {
    static let pageListeners = true
    static let taskHost = true
}

func tacOnOwnerThread() -> Bool { true }

func tacEventFailure() { print("Tachyon web service event delivery failed.") }

@_extern(wasm, module: "tachyon", name: "tac_host_emit")
func tacHostEmit(_ pointer: UnsafePointer<UInt8>?, _ length: Int32)

enum TacWebExports {
    static func sendToPage(_ payload: String) {
        var text = payload
        text.withUTF8 { tacHostEmit($0.baseAddress, Int32($0.count)) }
    }

    static func copied(_ text: String) -> UnsafeMutablePointer<CChar> {
        let bytes = Array(text.utf8CString)
        let copy = UnsafeMutablePointer<CChar>.allocate(capacity: bytes.count)
        copy.initialize(from: bytes, count: bytes.count)
        return copy
    }
}

@_expose(wasm, "tac_wasm_alloc")
@_cdecl("tac_wasm_alloc")
func tacWasmAlloc(_ size: Int32) -> UnsafeMutableRawPointer {
    UnsafeMutableRawPointer.allocate(byteCount: Int(size), alignment: 1)
}

@_expose(wasm, "tac_wasm_dealloc")
@_cdecl("tac_wasm_dealloc")
func tacWasmDealloc(_ pointer: UnsafeMutableRawPointer, _ size: Int32) {
    pointer.deallocate()
}

@_expose(wasm, "tac_native_invoke")
@_cdecl("tac_native_invoke")
func tacWebInvoke(_ request: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar> {
    if TacBridge.emit == nil { TacBridge.emit = TacWebExports.sendToPage }
    return TacWebExports.copied(tacNativeInvoke(String(cString: request)))
}

@_expose(wasm, "tac_native_free")
@_cdecl("tac_native_free")
func tacWebFree(_ answer: UnsafeMutablePointer<CChar>) {
    answer.deallocate()
}

@_spi(ExperimentalCustomExecutors) import _Concurrency

func tacAwait<Value>(_ work: @escaping () async throws -> Value) throws -> Value {
    let awaited = TacAwaited<Value>()
    Task { await awaited.finish(work) }
    try MainActor.executor.runUntil { awaited.result != nil }
    return try awaited.value()
}

func tacBeginTask(_ owner: String, _ work: @escaping ([Any?]) async throws -> Any?, _ arguments: [Any?]) throws -> String {
    TacTasks.begin(owner, work, arguments)
}

func tacCancelTasks(_ owner: String?) {
    TacTasks.cancel(owner)
}

func tacCreateNativeOwner<T>(_ owner: String, _ work: () -> T) -> T {
    work()
}

func tacDisposeNativeOwner(_ owner: String) {}
