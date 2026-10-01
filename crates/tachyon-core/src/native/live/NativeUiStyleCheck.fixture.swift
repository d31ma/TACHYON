import Foundation
import Darwin
import AppKit
import WebKit

@main struct NativeUiStyleCheck {
    @MainActor static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        Task { @MainActor in
            do {
                try await run(CommandLine.arguments)
                app.terminate(nil)
            } catch {
                FileHandle.standardError.write(Data("Native stylesheet fixture failed: \(error)\n".utf8))
                Darwin.exit(1)
            }
        }
        app.run()
    }

    @MainActor private static func run(_ arguments: [String]) async throws {
        switch arguments.count {
        case 2: try css()
        case 7: try await ui(arguments)
        default: throw NativeUiRuntimeFailure(description: "Invalid stylesheet fixture arguments")
        }
    }

    private static func css() throws {
        try NativeUiCssProfile.validate(Data("/* @import url('no') */ main::before { content: '@import url(ignored)'; color: blue; }".utf8))
        try NativeUiCssProfile.validate(Data("main { content: 'escaped \\\" quote'; margin: 2px; }".utf8))
        let invalid = ["@import 'remote.css';", "\\75rl('x');", "main { background: url('x'); }", "main { background: image-set('x' 1x); }", "/*/", "main { content: 'first\n@import'; }"]
        try invalid.forEach { try refusedCss($0) }
        try NativeUiRuntimeFailure.require(!NativeUiStylePhase.invalidating(nil).mayPrepare(), "New preparation was allowed during invalidation")
        print("native-ui-styles-css-profile-passed")
    }

    private static func refusedCss(_ source: String) throws {
        let failure = cssFailure(source)
        try NativeUiRuntimeFailure.require(failure != nil, "Unsupported CSS was accepted")
    }

    private static func cssFailure(_ source: String) -> Error? {
        do { try NativeUiCssProfile.validate(Data(source.utf8)); return nil } catch { return error }
    }

    @MainActor private static func ui(_ arguments: [String]) async throws {
        let installed = try generation(arguments[1], arguments[2], arguments[3])
        let candidate = try generation(arguments[4], arguments[5], arguments[6])
        try Data("main { color: red; }".utf8).write(to: URL(fileURLWithPath: arguments[4]).appendingPathComponent("styles/theme.css"))
        let scene = NativeUiStyleScene(installed)
        try await scene.start()
        try await initialized(scene)
        let marker = try await scene.page("document.querySelector('#field').value = 'kept'; document.querySelector('#field').focus(); return window.marker")
        let window = scene.window.windowNumber
        let identity = request("identity")
        let pending = Task { @MainActor in try await scene.page("await new Promise(resolve => setTimeout(resolve, 250)); return 'reply-kept'") }
        let ticket = try await scene.session.prepare(candidate)
        try await scene.session.commit(ticket)
        try await committed(scene, marker, identity)
        try NativeUiRuntimeFailure.require(try await pending.value == "reply-kept", "Pending WebKit reply was interrupted")
        try NativeUiRuntimeFailure.require(scene.window.windowNumber == window && scene.window.contentView === scene.view, "Owned window/view changed")
        try await scene.session.commit(ticket)
        try NativeUiRuntimeFailure.require(try await scene.page("return String(document.querySelectorAll('link[rel=stylesheet]').length)") == "2", "Commit duplicated stylesheet nodes")
        try await scene.session.rollback(ticket)
        try NativeUiRuntimeFailure.require(try await scene.color() == "rgb(0, 0, 0)", "Rollback did not restore original CSS")
        try await staleInvalidate(scene, candidate)
        try await failedCommit(scene, candidate)
        try await failedLoad(scene, candidate)
        try isolated(scene)
        await scene.session.invalidate()
        try NativeUiRuntimeFailure.require(LiveCounterService.starts == 1 && LiveCounterService.stops == 0, "CSS updated native lifecycle")
        _ = tacNativeInvoke("{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"6F9619FF-8B86-D011-B42D-00C04FC964FF\",\"op\":\"dispose\"}")
        try NativeUiRuntimeFailure.require(LiveCounterService.stops == 1, "Explicit final disposal must stop exactly once")
        scene.window.close()
        print("native-ui-styles-webkit-passed pid=\(ProcessInfo.processInfo.processIdentifier) window=\(window)")
    }

    @MainActor private static func initialized(_ scene: NativeUiStyleScene) async throws {
        let answer = tacNativeInvoke("{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"6F9619FF-8B86-D011-B42D-00C04FC964FF\",\"op\":\"init\"}")
        try NativeUiRuntimeFailure.require(!answer.contains("error"), "Native service did not initialize")
        try NativeUiRuntimeFailure.require(request("increment") == "{\"value\":1}", "Native count did not increment")
        try NativeUiRuntimeFailure.require(try await scene.color() == "rgb(0, 0, 0)", "Initial stylesheet is not black")
    }

    @MainActor private static func committed(_ scene: NativeUiStyleScene, _ marker: String, _ identity: String) async throws {
        try NativeUiRuntimeFailure.require(try await scene.color() == "rgb(0, 0, 255)", "Captured candidate CSS is not blue")
        try NativeUiRuntimeFailure.require(request("identity") == identity && request("read") == "{\"value\":1}", "Actual native root identity/state changed")
        let state = try await scene.page("document.querySelector('#click').click(); return JSON.stringify([window.marker, document.querySelector('#field').value, document.activeElement.id, window.events])")
        let expected = try JSONSerialization.data(withJSONObject: [marker, "kept", "field", 1], options: [])
        try NativeUiRuntimeFailure.require(state == String(decoding: expected, as: UTF8.self), "Document/input/focus or event delivery changed")
        try NativeUiRuntimeFailure.require(try await scene.page("return typeof globalThis.__tachyonNativeStyles") == "undefined", "Private stylesheet world leaked to page")
    }

    @MainActor private static func staleInvalidate(_ scene: NativeUiStyleScene, _ candidate: NativeUiRuntimeGeneration) async throws {
        let previous = try await scene.session.prepare(candidate)
        await scene.session.invalidate()
        let current = try await scene.session.prepare(candidate)
        let failure = await invalidateFailure(scene, previous)
        try NativeUiRuntimeFailure.require(failure != nil, "Delayed old invalidation was accepted")
        try await scene.session.commit(current)
        try NativeUiRuntimeFailure.require(try await scene.color() == "rgb(0, 0, 255)", "Old invalidation changed new stylesheet transaction")
        try await scene.session.rollback(current)
        try NativeUiRuntimeFailure.require(try await scene.color() == "rgb(0, 0, 0)", "New transaction rollback failed after stale invalidation")
    }

    @MainActor private static func invalidateFailure(_ scene: NativeUiStyleScene, _ ticket: NativeUiStyleTicket) async -> Error? {
        do { _ = try await NativeUiStyleCall.run(scene.view, "invalidate", ticket); return nil } catch { return error }
    }

    @MainActor private static func failedCommit(_ scene: NativeUiStyleScene, _ candidate: NativeUiRuntimeGeneration) async throws {
        let ticket = try await scene.session.prepare(candidate)
        _ = try await scene.page("document.querySelector('link[href*=\".tachyon-native-ui\"]').media = 'print'; return 'mutated'")
        let error = await commitFailure(scene, ticket)
        try NativeUiRuntimeFailure.require(error != nil, "Mutation was silently committed")
        try await scene.session.reconcile(ticket)
        try NativeUiRuntimeFailure.require(try await scene.color() == "rgb(0, 0, 0)", "Failed commit did not restore original CSS")
    }

    @MainActor private static func commitFailure(_ scene: NativeUiStyleScene, _ ticket: NativeUiStyleTicket) async -> Error? {
        do { try await scene.session.commit(ticket); return nil } catch { return error }
    }

    @MainActor private static func failedLoad(_ scene: NativeUiStyleScene, _ candidate: NativeUiRuntimeGeneration) async throws {
        try scene.assets.stage(candidate)
        let url = try NativeUiStyleAddress.url(String(repeating: "b", count: 64), "styles/theme.css")
        let styles = ["styles/theme.css": ["previous": try NativeUiStyleAddress.url(scene.assets.current.descriptor.generation, "styles/theme.css").absoluteString, "candidate": url.absoluteString]]
        let ticket = NativeUiStyleTicket(token: UUID().uuidString, epoch: 0, url: scene.view.url!.absoluteString, generation: candidate.descriptor.generation)
        let error = await loadFailure(scene, ticket, styles)
        try NativeUiRuntimeFailure.require(error != nil, "Unknown generation loaded")
        scene.assets.discard()
        try NativeUiRuntimeFailure.require(try await scene.color() == "rgb(0, 0, 0)", "Load failure changed active styles")
    }

    @MainActor private static func loadFailure(_ scene: NativeUiStyleScene, _ ticket: NativeUiStyleTicket, _ styles: [String: [String: String]]) async -> Error? {
        do { _ = try await NativeUiStyleCall.run(scene.view, "prepare", ticket, styles); return nil } catch { return error }
    }

    @MainActor private static func isolated(_ scene: NativeUiStyleScene) throws {
        let foreign = try NativeUiStyleAddress.url(String(repeating: "b", count: 64), "styles/theme.css")
        let error = assetFailure(scene, foreign)
        try NativeUiRuntimeFailure.require(error != nil, "Unknown generation was served")
    }

    @MainActor private static func assetFailure(_ scene: NativeUiStyleScene, _ url: URL) -> Error? {
        do { _ = try scene.assets.asset(url); return nil } catch { return error }
    }

    private static func generation(_ path: String, _ descriptor: String, _ fingerprint: String) throws -> NativeUiRuntimeGeneration {
        let file = open(path, O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC)
        try NativeUiRuntimeFailure.require(file >= 0, "Cannot open test-owned capture")
        defer { Darwin.close(file) }
        return try NativeUiRuntimeGeneration.trusted(file, Data(contentsOf: URL(fileURLWithPath: descriptor)), fingerprint)
    }

    @MainActor private static func request(_ method: String) -> String {
        tacNativeInvoke("{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"6F9619FF-8B86-D011-B42D-00C04FC964FF\",\"op\":\"call\",\"name\":\"\(method)\",\"args\":[]}")
    }
}
