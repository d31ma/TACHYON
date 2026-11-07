@_extern(wasm, module: "tachyon", name: "tac_host_storage")
func tacHostStorage(_ pointer: UnsafePointer<UInt8>?, _ length: Int32) -> UnsafeMutablePointer<CChar>?

enum TacWebStorage {
    static func ask(_ request: String) -> String? {
        var text = request
        guard let answer = text.withUTF8({ tacHostStorage($0.baseAddress, Int32($0.count)) }) else { return nil }
        defer { answer.deallocate() }
        return String(cString: answer)
    }
}
