import Foundation

struct NativeUiStyleTicket: Equatable {
    let token: String
    let epoch: UInt64
    let url: String
    let generation: String

    func arguments() -> [String: Any] {
        ["token": token, "epoch": NSNumber(value: epoch), "url": url, "generation": generation]
    }
}
