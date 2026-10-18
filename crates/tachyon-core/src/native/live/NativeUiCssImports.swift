import Foundation

final class NativeUiCssImports {
    private static let depthLimit = 16
    private let graph: [String: [String]]
    private var heights: [String: Int] = [:]
    private var active: Set<String> = []

    private init(_ graph: [String: [String]]) { self.graph = graph }

    static func validate(_ graph: [String: [String]]) throws {
        let walker = NativeUiCssImports(graph)
        try graph.keys.sorted().forEach { try walker.root($0) }
    }

    private func root(_ path: String) throws {
        let chain = try height(path, 0)
        try NativeUiRuntimeFailure.require(chain <= Self.depthLimit, "CSS @import depth bound exceeded")
    }

    private func height(_ path: String, _ depth: Int) throws -> Int {
        try NativeUiRuntimeFailure.require(depth < Self.depthLimit, "CSS @import depth bound exceeded")
        return try heights[path] ?? measured(path, depth)
    }

    private func measured(_ path: String, _ depth: Int) throws -> Int {
        try NativeUiRuntimeFailure.require(active.insert(path).inserted, "CSS @import cycle")
        let deepest = try (graph[path] ?? []).map { try height($0, depth + 1) }.max() ?? 0
        active.remove(path)
        heights[path] = deepest + 1
        return deepest + 1
    }
}
