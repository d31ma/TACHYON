import Foundation

final class NativeUiRuntimeCapture {
    private(set) var assets: [String: Data] = [:]
    private var entries = 0
    private var total = 0

    static func read(_ directory: Int32) throws -> [String: Data] {
        let capture = NativeUiRuntimeCapture()
        let root = try NativeUiRuntimeDirectory(borrowing: directory)
        try capture.visit(root, "", 0)
        try NativeUiRuntimeFailure.require(!capture.assets.isEmpty, "Asset generation is empty")
        return capture.assets
    }

    private func visit(_ root: NativeUiRuntimeDirectory, _ path: String, _ depth: Int) throws {
        try NativeUiRuntimeFailure.require(depth <= NativeUiRuntimeBounds.depth, "Directory depth limit exceeded")
        let names = try root.names().filter { ![".", ".."].contains($0) }
        try names.forEach { try entry(root, path, ($0, depth)) }
    }

    private func entry(_ root: NativeUiRuntimeDirectory, _ parent: String, _ position: (String, Int)) throws {
        entries += 1
        try NativeUiRuntimeFailure.require(entries <= NativeUiRuntimeBounds.entries, "Directory entry limit exceeded")
        let path = [parent, position.0].filter { !$0.isEmpty }.joined(separator: "/")
        try NativeUiRuntimeAsset.canonicalPath(path)
        try found(root, path, position)
    }

    private func found(_ root: NativeUiRuntimeDirectory, _ path: String, _ position: (String, Int)) throws {
        if try root.isDirectory(position.0) {
            try visit(root.child(position.0), path, position.1 + 1)
        } else {
            let bytes = try NativeUiRuntimeFile.read(root.descriptor, position.0)
            try captured(path, bytes)
        }
    }

    private func captured(_ path: String, _ bytes: Data) throws {
        total += bytes.count
        try NativeUiRuntimeFailure.require(total <= NativeUiRuntimeBounds.totalBytes, "Capture byte limit exceeded")
        try NativeUiRuntimeFailure.require(assets.count < NativeUiRuntimeBounds.files, "Asset count limit exceeded")
        try NativeUiRuntimeFailure.require(assets[path] == nil, "Ambiguous asset identity")
        assets[path] = bytes
    }
}
