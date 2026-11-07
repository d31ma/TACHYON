import Foundation

struct NativeUiRuntimeFailure: Error, CustomStringConvertible {
    let description: String

    static func require(_ valid: Bool, _ message: String) throws {
        guard valid else { throw NativeUiRuntimeFailure(description: message) }
    }
}
