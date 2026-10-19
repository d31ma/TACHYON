private enum TacDeepLink {
    static let schemes: Set<String> = __LINK_SCHEMES__
    static let hosts: Set<String> = __LINK_HOSTS__
    static let limit = 2048

    static func target(_ link: String) -> String? {
        guard let (path, suffix) = parts(link) else { return nil }
        let route = "/" + path
        guard tachyonRoute(route) != nil else { return "tachyon-app://bundle/" + tachyonEntryDocument }
        return "tachyon-app://bundle" + route + suffix
    }

    private static func parts(_ link: String) -> (String, String)? {
        let bytes = Array(link.utf8)
        guard !bytes.isEmpty, bytes.count <= limit,
              bytes.allSatisfy({ $0 > 32 && $0 < 127 && $0 != 34 && $0 != 92 }),
              let colon = link.firstIndex(of: ":"), colon != link.startIndex else { return nil }
        let scheme = link[..<colon].lowercased()
        var rest = link[link.index(after: colon)...]
        if schemes.contains(scheme) {
            rest = rest.drop(while: { $0 == "/" })
        } else if scheme == "https" && rest.hasPrefix("//") {
            let authority = rest.dropFirst(2)
            let host = authority.prefix(while: { !"/?#".contains($0) })
            guard hosts.contains(host.lowercased()) else { return nil }
            rest = authority.dropFirst(host.count)
            if rest.hasPrefix("/") { rest = rest.dropFirst() }
        } else {
            return nil
        }
        var path = rest.prefix(while: { !"?#".contains($0) })
        let suffix = rest.dropFirst(path.count)
        while path.hasSuffix("/") { path = path.dropLast() }
        return (String(path), String(suffix))
    }
}
