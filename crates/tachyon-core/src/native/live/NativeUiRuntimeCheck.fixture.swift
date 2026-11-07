import Foundation
import Darwin

@main struct NativeUiRuntimeCheck {
    static func main() throws { try execute(CommandLine.arguments) }

    private static func execute(_ arguments: [String]) throws {
        switch arguments.count {
        case 5: try single(arguments)
        case 8: try pair(arguments)
        default: throw NativeUiRuntimeFailure(description: "Invalid fixture arguments")
        }
    }

    private static func single(_ arguments: [String]) throws {
        let source = try Data(contentsOf: URL(fileURLWithPath: arguments[3]))
        let descriptor = open(arguments[2], O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)
        try NativeUiRuntimeFailure.require(descriptor >= 0, "Cannot open test-owned directory")
        defer { Darwin.close(descriptor) }
        try run(arguments[1], descriptor, source, arguments[4])
    }

    private static func pair(_ arguments: [String]) throws {
        let installed = try generation(arguments[2], arguments[3], arguments[4])
        let candidate = try generation(arguments[5], arguments[6], arguments[7])
        let expected: [String: NativeUiRuntimeChange] = ["styles": .prepareStyles, "restart": .restartRequired]
        try NativeUiRuntimeFailure.require(expected[arguments[1]] == candidate.compare(installed), "Incorrect compatible/restart classification")
        print("native-ui-runtime-classification-passed")
    }

    private static func generation(_ folder: String, _ path: String, _ fingerprint: String) throws -> NativeUiRuntimeGeneration {
        let source = try Data(contentsOf: URL(fileURLWithPath: path))
        let descriptor = open(folder, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)
        try NativeUiRuntimeFailure.require(descriptor >= 0, "Cannot open test-owned directory")
        defer { Darwin.close(descriptor) }
        return try NativeUiRuntimeGeneration.trusted(descriptor, source, fingerprint)
    }

    private static func run(_ mode: String, _ directory: Int32, _ source: Data, _ fingerprint: String) throws {
        switch mode {
        case "valid": try valid(directory, source, fingerprint)
        case "retained": try retained(directory, source, fingerprint)
        case "refuse": try refused(directory, source, fingerprint)
        default: throw NativeUiRuntimeFailure(description: "Unknown fixture mode")
        }
    }

    private static func valid(_ directory: Int32, _ source: Data, _ fingerprint: String) throws {
        let generation = try NativeUiRuntimeGeneration.trusted(directory, source, fingerprint)
        try NativeUiRuntimeFailure.require(generation.descriptor.canonical() == source, "Rust/Swift canonical bytes differ")
        try NativeUiRuntimeFailure.require(generation.compare(generation) == .unchanged, "Installed generation changed")
        print("native-ui-runtime-vector-passed")
    }

    private static func retained(_ directory: Int32, _ source: Data, _ fingerprint: String) throws {
        let generation = try NativeUiRuntimeGeneration.trusted(directory, source, fingerprint)
        let original = generation.asset("styles/theme.css")
        let descriptor = openat(directory, "styles/theme.css", O_WRONLY | O_TRUNC | O_NOFOLLOW | O_CLOEXEC)
        try NativeUiRuntimeFailure.require(descriptor >= 0, "Cannot mutate test-owned fixture")
        Darwin.close(descriptor)
        try NativeUiRuntimeFailure.require(generation.asset("styles/theme.css") == original, "Capture reread mutable disk bytes")
        try refused(directory, source, fingerprint)
        print("native-ui-runtime-retained-passed")
    }

    private static func refused(_ directory: Int32, _ source: Data, _ fingerprint: String) throws {
        let error = refusal(directory, source, fingerprint)
        try NativeUiRuntimeFailure.require(error != nil, "Invalid generation accepted")
        print("native-ui-runtime-refused")
    }

    private static func refusal(_ directory: Int32, _ source: Data, _ fingerprint: String) -> Error? {
        do {
            _ = try NativeUiRuntimeGeneration.trusted(directory, source, fingerprint)
            return nil
        } catch { return error }
    }
}
