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
        case 8: try retainedStore(arguments)
        default: throw NativeUiRuntimeFailure(description: "Invalid stylesheet fixture arguments")
        }
    }

    private static let controlSheet = NativeUiCssSheet(
        path: "styles/theme.css",
        assets: ["index.html", "styles/theme.css", "styles/parts/base.css", "images/dot.svg", "fonts/body.woff2"],
        stylesheets: ["styles/theme.css", "styles/parts/base.css"]
    )

    private static func css() throws {
        try acceptedCss("/* @import url('no') */ main::before { content: '@import url(ignored)'; color: blue; }")
        try acceptedCss("main { content: 'escaped \\\" quote'; margin: 2px; }")
        let invalid = ["@import 'remote.css';", "\\75rl('x');", "main { background: url('x'); }", "main { background: image-set('x' 1x); }", "/*/", "main { content: 'first\n@import'; }"]
        try invalid.forEach { try refusedCss($0) }
        try grammar()
        try resourceGrammar()
        try importGraph()
        try NativeUiRuntimeFailure.require(!NativeUiStylePhase.invalidating(nil).mayPrepare(), "New preparation was allowed during invalidation")
        print("native-ui-styles-css-profile-passed")
    }

    private static func grammar() throws {
        let sources = [
            "@view-transition { navigation: auto; } tachyon-component, tachyon-island { display: block; }",
            "@layer brand { @media (min-width: 800px) { main:not(.hidden) { width: clamp(1rem, calc(20px + 1vw), 2rem); color: rgb(10 20 30); } } }",
            "@supports (display: grid) { main { grid-template-columns: repeat(2, minmax(0, 1fr)); background: radial-gradient(red, blue); } }",
            "@container sidebar (width > 10px) { main { transform: translateX(2px); } } @keyframes pulse { from { opacity: .5; } to { opacity: 1; } }",
            "@container telemetry/**/ (min-width: 10px) { main { width: max(10px, 20px); } } @media screen and (min-width: 800px) { main { color: blue; } }",
            "@property --color { syntax: '<color>'; initial-value: red; inherits: false; } main { background: var(--outside); }",
        ]
        try sources.forEach { try acceptedCss($0) }
        let refused = [
            "@font-face { src: url('unknown.woff'); }", "@namespace url('outside');",
            "main { background: URL('outside'); }", "main { background: ur/**/l('outside'); }", "main { background: image-set('outside' 1x); }",
            "main { background: attr(data-image); }", "main { width: env(unknown); }", "main { width: unknown(1); }", "@/**/media { main { color: red; } }",
            "main { width: calc(1px; }", "main { color: blue; } }", "main { width: " + String(repeating: "(", count: 65) + "1" + String(repeating: ")", count: 65) + "; }",
        ]
        try refused.forEach { try refusedCss($0) }
    }

    private static func resourceGrammar() throws {
        let sources = [
            #"@import url("parts/base.css") layer(base) supports(display: grid) screen and (min-width: 10px); @import 'parts/base.css'; @IMPORT URL(./parts/base.css); main { color: red; }"#,
            #"main { background: url(../images/dot.svg) no-repeat, url( './../images/dot.svg' ); mask: url("../images/dot.svg"); filter: url(#soft-edge); }"#,
            #"@font-face { font-family: Body; src: url("../fonts/body.woff2") format("woff2"); } main { font-family: Body, serif; }"#,
            #":root { --ink: rgb(1 2 3); } main { color: var(--ink); margin: var( --gap , calc(1px + var(--edge, 2px)) ); }"#,
            #"@property --gap { syntax: '<length>'; inherits: false; initial-value: 3px; } @property /**/ --tone /**/ { syntax: '*'; inherits: true; }"#,
            #".md\:flex, .w-1\/2, .\31 0, .a\(b\), #\@id, .\E9 t\E9 , .a\\b, .\"quoted\" { display: flex; } .hover\:bg-blue:hover { color: blue; }"#,
            #"main::before { content: "url(../outside)"; } main { --url: 'https://example.com'; }"#,
        ]
        try sources.forEach { try acceptedCss($0) }
        let refused = [
            #"@import url("https://example.com/x.css");"#, #"@import "/styles/parts/base.css";"#, #"@import "../../escape.css";"#,
            #"@import "../images/dot.svg";"#, #"@import url(parts/base.css)"#, #"main { @import 'parts/base.css'; }"#, #"@import parts/base.css;"#,
            #"@import "parts/base.css" { }"#, #"@import url("parts/base.css" x);"#,
            #"main { background: url(//example.com/a.png); }"#, #"main { background: url(https://example.com/a.png); }"#,
            #"main { background: url(data:image/png;base64,AA==); }"#, #"main { background: url("../images/dot.svg?v=1"); }"#,
            #"main { background: url(../images/dot.svg#frag); }"#, #"main { background: url(../images/%64ot.svg); }"#,
            #"main { background: url(../images/dot.svg x); }"#, #"main { background: url("../images/dot.svg" x); }"#,
            #"main { background: url(../images\/dot.svg); }"#, #"main { background: url("../images\/dot.svg"); }"#,
            #"main { background: url(../../../../images/dot.svg); }"#, #"main { background: url(../images//dot.svg); }"#,
            #"main { background: url(../images/); }"#, #"main { background: url(); }"#, #"main { background: url(""); }"#,
            #"main { background: url(../images/dot.svg/*x*/); }"#, #"main { background: u\72l(../images/dot.svg); }"#,
            #"main { background: url(#); }"#, #"main { background: url(#a.b); }"#, #"main { background: url(../IMAGES/dot.svg); }"#,
            #"main { background: url('../images/dot.svg); }"#, #"main { background: url(../images/dot.svg; }"#, #"#url(https://example.com/a.png) { color: red; }"#,
            #"main { color: var(outside); }"#, #"main { color: var(); }"#, #"main { color: var(--); }"#, #"main { color: var(--a b); }"#,
            #"@property color { syntax: '*'; }"#, #"@property --a, --b { syntax: '*'; }"#, #"@font-face rule { src: local(Body); }"#,
            #"main { color: blue; } .a\"#, "main\\\n{ color: red; }", #"@\69mport 'parts/base.css';"#,
        ]
        try refused.forEach { try refusedCss($0) }
    }

    private static func importGraph() throws {
        try NativeUiCssImports.validate(["a.css": ["b.css", "c.css"], "b.css": ["c.css"], "c.css": []])
        let chain = (0..<16).map { "s\($0).css" }
        try NativeUiCssImports.validate(Dictionary(uniqueKeysWithValues: chain.indices.map { (chain[$0], Array(chain.dropFirst($0 + 1).prefix(1))) }))
        let graphs: [[String: [String]]] = [
            ["a.css": ["a.css"]], ["a.css": ["b.css"], "b.css": ["c.css"], "c.css": ["a.css"]],
            Dictionary(uniqueKeysWithValues: (chain + ["s16.css"]).indices.map { ((chain + ["s16.css"])[$0], Array((chain + ["s16.css"]).dropFirst($0 + 1).prefix(1))) }),
        ]
        try graphs.forEach { graph in try NativeUiRuntimeFailure.require(importFailure(graph) != nil, "Unbounded CSS import graph was accepted: \(graph.count)") }
    }

    private static func importFailure(_ graph: [String: [String]]) -> Error? {
        do { try NativeUiCssImports.validate(graph); return nil } catch { return error }
    }

    private static func acceptedCss(_ source: String) throws {
        do { _ = try NativeUiCssProfile.validate(Data(source.utf8), controlSheet) } catch {
            throw NativeUiRuntimeFailure(description: "Supported CSS was refused: \(source): \(error)")
        }
    }

    private static func refusedCss(_ source: String) throws {
        let failure = cssFailure(source)
        try NativeUiRuntimeFailure.require(failure != nil, "Unsupported CSS was accepted: \(source)")
    }

    private static func cssFailure(_ source: String) -> Error? {
        do { _ = try NativeUiCssProfile.validate(Data(source.utf8), controlSheet); return nil } catch { return error }
    }

    @MainActor private static func retainedStore(_ arguments: [String]) throws {
        let installed = try generation(arguments[2], arguments[3], arguments[4])
        let candidate = try generation(arguments[5], arguments[6], arguments[7])
        let assets = NativeUiStyleAssets(installed)
        let root = URL(string: "tachyon-app://bundle/styles/theme.css")!
        let old = try NativeUiStyleAddress.url(installed.descriptor.generation, "styles/theme.css")
        let new = try NativeUiStyleAddress.url(candidate.descriptor.generation, "styles/theme.css")
        try assets.stage(candidate)
        try assets.promote(candidate.descriptor.generation)
        try assets.prepareRollback(candidate.descriptor.generation)
        try NativeUiRuntimeFailure.require(try assets.asset(root) == installed.asset("styles/theme.css"), "Pending rollback served candidate at old root")
        try NativeUiRuntimeFailure.require(try assets.asset(old) == installed.asset("styles/theme.css") && assets.asset(new) == candidate.asset("styles/theme.css"), "Pending rollback retired a generation before acknowledgement")
        try assets.promote(candidate.descriptor.generation)
        try NativeUiRuntimeFailure.require(try assets.asset(root) == candidate.asset("styles/theme.css") && assets.asset(old) == installed.asset("styles/theme.css"), "Committed reconciliation did not restore candidate with retained rollback")
        try assets.prepareRollback(candidate.descriptor.generation)
        try assets.rollback(candidate.descriptor.generation)
        try NativeUiRuntimeFailure.require(try assets.asset(root) == installed.asset("styles/theme.css") && assets.asset(old) == installed.asset("styles/theme.css"), "Acknowledged rollback lost installed capture")
        try NativeUiRuntimeFailure.require(storeFailure(assets, new) != nil, "Acknowledged rollback did not retire candidate")
        print("native-ui-styles-retained-rollback-passed")
    }

    @MainActor private static func storeFailure(_ assets: NativeUiStyleAssets, _ url: URL) -> Error? {
        do { _ = try assets.asset(url); return nil } catch { return error }
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
        let beforeRollback = try await scene.stylesheetSnapshot()
        try await scene.session.rollback(ticket)
        let afterRollback = try await scene.stylesheetSnapshot()
        try NativeUiRuntimeFailure.require(try await scene.color() == "rgb(0, 0, 0)", "Rollback did not restore original CSS; before=\(beforeRollback); after=\(afterRollback)")
        try await staleInvalidate(scene, candidate)
        try await failedCommit(scene, candidate)
        try await failedLoad(scene, candidate)
        try isolated(scene)
        await scene.session.invalidate()
        try NativeUiRuntimeFailure.require(request("started") == "{\"value\":1}" && request("stopped") == "{\"value\":0}", "CSS updated native lifecycle")
        _ = tacNativeInvoke("{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"6F9619FF-8B86-D011-B42D-00C04FC964FF\",\"op\":\"dispose\"}")
        try observedStop()

        scene.window.close()
        print("native-ui-styles-webkit-passed pid=\(ProcessInfo.processInfo.processIdentifier) window=\(window)")
    }

    @MainActor private static func observedStop() throws {
        let owner = "6F9619FF-8B86-D011-B42D-00C04FC964FE"
        let answer = tacNativeInvoke("{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"\(owner)\",\"op\":\"init\"}")
        try NativeUiRuntimeFailure.require(!answer.contains("error"), "Lifecycle observer did not initialize")
        try NativeUiRuntimeFailure.require(request("stopped", owner) == "{\"value\":1}", "Explicit final disposal must stop exactly once")
        _ = tacNativeInvoke("{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"\(owner)\",\"op\":\"dispose\"}")
    }

    @MainActor private static func initialized(_ scene: NativeUiStyleScene) async throws {
        let answer = tacNativeInvoke("{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"6F9619FF-8B86-D011-B42D-00C04FC964FF\",\"op\":\"init\"}")
        try NativeUiRuntimeFailure.require(!answer.contains("error"), "Native service did not initialize")
        try NativeUiRuntimeFailure.require(request("increment") == "{\"value\":1}", "Native count did not increment")
        try NativeUiRuntimeFailure.require(try await scene.color() == "rgb(0, 0, 0)", "Initial stylesheet is not black")
    }

    @MainActor private static func committed(_ scene: NativeUiStyleScene, _ marker: String, _ identity: String) async throws {
        try NativeUiRuntimeFailure.require(try await scene.color() == "rgb(0, 0, 255)", "Captured candidate CSS is not blue")
        let retained = try await scene.page("const original=document.querySelectorAll('link[rel=stylesheet]')[0]; return String(!original.disabled && original.media==='not all' && original.sheet?.cssRules[0]?.cssText==='main { color: black; }')")
        try NativeUiRuntimeFailure.require(retained == "true", "Original sheet was not retained loaded and inactive")
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

    @MainActor private static func request(_ method: String, _ owner: String = "6F9619FF-8B86-D011-B42D-00C04FC964FF") -> String {
        tacNativeInvoke("{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"\(owner)\",\"op\":\"call\",\"name\":\"\(method)\",\"args\":[]}")
    }
}
