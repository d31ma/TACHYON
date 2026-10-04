struct MacOsDragRegions {
    static let limit = 256
    static let largestEdge = 100_000.0
    static let empty = MacOsDragRegions(drag: [], exclude: [])

    let drag: [NSRect]
    let exclude: [NSRect]

    static func parse(_ request: [String: Any]) -> MacOsDragRegions? {
        guard let drag = rectangles(request["drag"]), let exclude = rectangles(request["exclude"]) else {
            return nil
        }
        return MacOsDragRegions(drag: drag, exclude: exclude)
    }

    func contains(_ point: NSPoint) -> Bool {
        let insideDrag = drag.contains { (rectangle: NSRect) -> Bool in rectangle.contains(point) }
        guard insideDrag else { return false }
        return !exclude.contains { (rectangle: NSRect) -> Bool in rectangle.contains(point) }
    }

    private static func rectangles(_ value: Any?) -> [NSRect]? {
        guard let list = value as? [Any], list.count <= limit else { return nil }
        var result: [NSRect] = []
        for item in list {
            guard let rectangle = rectangle(item) else { return nil }
            result.append(rectangle)
        }
        return result
    }

    private static func rectangle(_ item: Any) -> NSRect? {
        guard let numbers = item as? [NSNumber], numbers.count == 4 else { return nil }
        let values = numbers.map { (number: NSNumber) -> Double in number.doubleValue }
        let bounded = values.allSatisfy { (value: Double) -> Bool in value.isFinite && abs(value) <= largestEdge }
        guard bounded, values[2] >= 0, values[3] >= 0 else { return nil }
        return NSRect(x: values[0], y: values[1], width: values[2], height: values[3])
    }
}

