private struct TacServiceKey: Hashable {
    let service: String
    let owner: String
}

private final class TacServiceNode {
    let members: [String: TacMember]
    let capabilities: String
    private var started = false
    private var disposed = false

    init(_ members: [String: TacMember], _ capabilities: String) {
        self.members = members
        self.capabilities = capabilities
    }

    func start() throws {
        guard !started else { return }
        started = true
        for (_, member) in members.sorted(by: { $0.key < $1.key }) {
            if case .hook(false, let invoke) = member { try invoke() }
        }
    }

    func dispose() {
        guard !disposed else { return }
        disposed = true
        if started {
            for (_, member) in members.sorted(by: { $0.key < $1.key }) {
                if case .hook(true, let invoke) = member { do { try invoke() } catch { tacEventFailure() } }
            }
        }
        tacDisposeNativeOwner(capabilities)
    }
}

private final class TacServicePage {
    var nodes: [TacServiceKey: TacServiceNode] = [:]
    var order: [TacServiceKey] = []

    func take(_ keys: [TacServiceKey]) -> [TacServiceNode] {
        order.removeAll { keys.contains($0) }
        return keys.compactMap { nodes.removeValue(forKey: $0) }
    }
}

// A page singleton is one node per route and service; a prototype is one node per owner.
private enum TacServicePages {
    static let providerPrefix = "00000000-0000-4000-a000-"
    static let nodeLimit = 4096
    nonisolated(unsafe) private static var pages: [String: TacServicePage] = [:]
    nonisolated(unsafe) private static var providers = 0

    static func reserved(_ owner: String) -> Bool { owner.lowercased().hasPrefix(providerPrefix) }

    static func full() -> Bool { pages.values.reduce(0) { $0 + $1.nodes.count } >= nodeLimit }

    static func obtain(_ route: String, _ service: String, _ owner: String, _ props: [String: Any]) -> TacServiceNode? {
        let shared = TacPlatform.pageServices && !tacServicePrototype(service)
        let key = TacServiceKey(service: service, owner: shared ? "" : owner)
        let page = pages[route] ?? TacServicePage()
        if let node = page.nodes[key] { return node }
        let capabilities = shared ? provider() : owner
        guard let members = tacCreateNativeOwner(capabilities, { tacServiceMembers(service, props) }) else {
            tacDisposeNativeOwner(capabilities)
            return nil
        }
        let node = TacServiceNode(members, capabilities)
        pages[route] = page
        page.nodes[key] = node
        page.order.append(key)
        return node
    }

    static func discard(_ route: String, _ node: TacServiceNode) {
        guard let page = pages[route] else { return }
        for taken in page.take(page.order.filter { page.nodes[$0] === node }) { taken.dispose() }
    }

    static func disposeOwner(_ owner: String) {
        for route in pages.keys.sorted() {
            guard let page = pages[route] else { continue }
            for node in page.take(page.order.reversed().filter { $0.owner == owner }) { node.dispose() }
        }
    }

    static func disposeAll() {
        let disposed = pages
        pages = [:]
        for route in disposed.keys.sorted() {
            guard let page = disposed[route] else { continue }
            for node in page.take(page.order.reversed()) { node.dispose() }
        }
    }

    private static func provider() -> String {
        providers += 1
        let identity = String(providers, radix: 16)
        return providerPrefix + String(repeating: "0", count: max(0, 12 - identity.count)) + identity
    }
}
