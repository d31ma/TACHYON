import Foundation

struct NativeUiCssSheet {
    let path: String
    let assets: Set<String>
    let stylesheets: Set<String>

    func folder() -> [Substring] {
        Array(path.split(separator: "/", omittingEmptySubsequences: false).dropLast())
    }

    func captured(_ path: String) -> Bool { assets.contains(path) }

    func stylesheet(_ path: String) -> Bool { stylesheets.contains(path) }
}
