import CoreGraphics
import Foundation

enum WindowBounds {
    static let normalLayer = 0

    static func width(of owner: String) -> Int? {
        let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
        return windows.lazy
            .filter { isNormalWindow($0, of: owner) }
            .compactMap { ($0[kCGWindowBounds as String] as? [String: Any])?["Width"] as? Double }
            .map { Int($0) }
            .first
    }

    static func isNormalWindow(_ window: [String: Any], of owner: String) -> Bool {
        let named = window[kCGWindowOwnerName as String] as? String == owner
        let normal = window[kCGWindowLayer as String] as? Int == normalLayer
        return named && normal
    }

    static func run() {
        guard CommandLine.arguments.count == 2, let width = width(of: CommandLine.arguments[1]) else {
            FileHandle.standardError.write(Data("window-grant bounds: no window\n".utf8))
            exit(1)
        }
        print(width)
    }
}

WindowBounds.run()
