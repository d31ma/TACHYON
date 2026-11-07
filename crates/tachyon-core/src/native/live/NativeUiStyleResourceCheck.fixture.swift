import Foundation
import Darwin
import AppKit
import WebKit

@main struct NativeUiStyleResourceCheck {
    @MainActor static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        Task { @MainActor in
            do {
                try await run(CommandLine.arguments)
                app.terminate(nil)
            } catch {
                FileHandle.standardError.write(Data("Native stylesheet resource fixture failed: \(error)\n".utf8))
                Darwin.exit(1)
            }
        }
        app.run()
    }

    @MainActor private static func run(_ arguments: [String]) async throws {
        try NativeUiRuntimeFailure.require(arguments.count == 7, "Invalid stylesheet resource fixture arguments")
        let installed = try generation(arguments[1], arguments[2], arguments[3])
        let candidate = try generation(arguments[4], arguments[5], arguments[6])
        let scene = NativeUiStyleScene(installed)
        try await scene.start()
        try await scene.requested("/styles/parts/tokens.css")
        try await scene.requested("/styles/parts/shape.css")
        try await scene.requested("/images/dot.svg")
        try await styled(scene, "rgb(0, 0, 0)|3px|50px|20px")
        let ticket = try await scene.session.prepare(candidate)
        try await scene.session.commit(ticket)
        let prefix = NativeUiStyleAddress.prefix + candidate.descriptor.generation + "/"
        try await scene.requested(prefix + "styles/parts/tokens.css")
        try await scene.requested(prefix + "styles/parts/shape.css")
        try await scene.requested(prefix + "images/dot.svg")
        try await styled(scene, "rgb(0, 0, 255)|7px|50px|20px")
        let image = try await scene.page("return String(getComputedStyle(document.querySelector('#sample'), '::before').content.includes('tachyon-app://bundle\(prefix)images/dot.svg'))")
        try NativeUiRuntimeFailure.require(image == "true", "Candidate url() did not resolve inside its captured generation")
        try await scene.session.rollback(ticket)
        try await styled(scene, "rgb(0, 0, 0)|3px|50px|20px")
        try contained(scene, installed, prefix)
        scene.window.close()
        print("native-ui-styles-resources-passed requests=\(scene.recorder.requests.count)")
    }

    @MainActor private static func styled(_ scene: NativeUiStyleScene, _ expected: String) async throws {
        let deadline = Date().addingTimeInterval(10)
        var style = try await computed(scene)
        while style != expected {
            try NativeUiRuntimeFailure.require(Date() < deadline, "Live resource stylesheet expected \(expected) but WebKit computed \(style)")
            try await Task.sleep(nanoseconds: 50_000_000)
            style = try await computed(scene)
        }
    }

    @MainActor private static func computed(_ scene: NativeUiStyleScene) async throws -> String {
        try await scene.page("const main=getComputedStyle(document.querySelector('#sample')); const item=getComputedStyle(document.querySelector('#escaped')); return [main.color, main.paddingLeft, item.width, item.height].join('|')")
    }

    @MainActor private static func contained(_ scene: NativeUiStyleScene, _ installed: NativeUiRuntimeGeneration, _ candidate: String) throws {
        let previous = NativeUiStyleAddress.prefix + installed.descriptor.generation + "/"
        let outside = scene.recorder.requests.filter { path in
            !path.hasPrefix(previous) && !path.hasPrefix(candidate) && installed.asset(String(path.dropFirst())) == nil
        }
        try NativeUiRuntimeFailure.require(outside.isEmpty && scene.recorder.failures.isEmpty, "Stylesheet resources left the captured generations: \(outside) \(scene.recorder.failures)")
    }

    private static func generation(_ path: String, _ descriptor: String, _ fingerprint: String) throws -> NativeUiRuntimeGeneration {
        let file = open(path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)
        try NativeUiRuntimeFailure.require(file >= 0, "Cannot open test-owned capture")
        defer { Darwin.close(file) }
        return try NativeUiRuntimeGeneration.trusted(file, Data(contentsOf: URL(fileURLWithPath: descriptor)), fingerprint)
    }
}
