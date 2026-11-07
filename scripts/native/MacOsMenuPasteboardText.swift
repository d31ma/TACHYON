@MainActor
final class MacOsMenuPasteboardText: NSTextView {
    var privateBoard: NSPasteboard?
    private(set) var writes = 0
    private(set) var reads = 0

    override func writeSelection(to pboard: NSPasteboard, types: [NSPasteboard.PasteboardType]) -> Bool {
        writes += 1
        guard let privateBoard else { return false }
        return super.writeSelection(to: privateBoard, types: types)
    }

    override func writeSelection(to pboard: NSPasteboard, type: NSPasteboard.PasteboardType) -> Bool {
        writes += 1
        guard let privateBoard else { return false }
        return super.writeSelection(to: privateBoard, type: type)
    }

    override func readSelection(from pboard: NSPasteboard) -> Bool {
        reads += 1
        guard let privateBoard else { return false }
        return super.readSelection(from: privateBoard)
    }

    override func readSelection(from pboard: NSPasteboard, type: NSPasteboard.PasteboardType) -> Bool {
        reads += 1
        guard let privateBoard else { return false }
        return super.readSelection(from: privateBoard, type: type)
    }
}
