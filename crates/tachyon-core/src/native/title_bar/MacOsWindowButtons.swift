@MainActor
enum MacOsWindowButtons {
    static let types: [NSWindow.ButtonType] = [.closeButton, .miniaturizeButton, .zoomButton]

    static func buttons(_ window: NSWindow) -> [NSButton] {
        types.compactMap { (type: NSWindow.ButtonType) -> NSButton? in window.standardWindowButton(type) }
    }

    static func extent(_ window: NSWindow) -> NSRect {
        var union = NSRect.null
        for button in buttons(window) where !button.isHidden {
            union = union.union(button.convert(button.bounds, to: nil))
        }
        return union
    }

    static func place(_ window: NSWindow, _ inset: NSPoint) {
        let buttons = buttons(window)
        guard buttons.count == types.count, !window.styleMask.contains(.fullScreen),
              let container = buttons[0].superview?.superview else { return }
        let size = buttons[0].frame.size
        let spacing = abs(buttons[1].frame.minX - buttons[0].frame.minX)
        let height = size.height + inset.y * 2
        container.frame = NSRect(x: container.frame.minX, y: window.frame.height - height,
                                 width: container.frame.width, height: height)
        let rightToLeft = window.windowTitlebarLayoutDirection == .rightToLeft
        for (index, button) in buttons.enumerated() {
            let offset = inset.x + CGFloat(index) * spacing
            let width = button.superview?.bounds.width ?? window.frame.width
            let x = rightToLeft ? width - offset - size.width : offset
            button.setFrameOrigin(NSPoint(x: x, y: inset.y))
        }
    }
}

