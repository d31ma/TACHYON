
enum Assert {
    struct Failure: Error, CustomStringConvertible {
        let description: String
    }

    static let nothingThrown = "Expected an error to be thrown, but nothing was thrown."
    static let messageSeparator = " ==> "

    static func equals<Value: Equatable>(_ expected: Value, _ actual: Value, _ message: String? = nil) throws {
        try check(expected == actual, "expected: <\(expected)> but was: <\(actual)>", message)
    }

    static func isTrue(_ condition: Bool, _ message: String? = nil) throws {
        try check(condition, "expected: <true> but was: <\(condition)>", message)
    }

    @discardableResult
    static func `throws`(_ action: () async throws -> Void, _ message: String? = nil) async throws -> Error {
        do {
            try await action()
        } catch {
            return error
        }
        throw Failure(description: text(nothingThrown, message))
    }

    private static func check(_ passed: Bool, _ failure: String, _ message: String?) throws {
        if !passed {
            throw Failure(description: text(failure, message))
        }
    }

    private static func text(_ failure: String, _ message: String?) -> String {
        guard let message else {
            return failure
        }
        return message + messageSeparator + failure
    }
}
