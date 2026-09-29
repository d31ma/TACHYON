@_extern(wasm, module: "tachyon", name: "tac_host_epoch_ms")
func tacHostEpochMilliseconds() -> Double

enum TacWebHost {
    static func epochMilliseconds() -> Int64 {
        Int64(tacHostEpochMilliseconds())
    }
}
