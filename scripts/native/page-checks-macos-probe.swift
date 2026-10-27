import AppKit
import ApplicationServices
import Foundation

enum PageChecksProbe {
    static let usage = "usage: page-checks-macos-probe <bundle-id> wait <text>... | within <seconds> <text>... | press <title> | click <title> | escape <seconds> <text>... | verdict <seconds> <passed> <failed> <prefix>... | quit | - allow <title>..."
    static let attempts = 120
    static let pause: TimeInterval = 0.25
    static let escapePause: TimeInterval = 1
    static let escapeKey: CGKeyCode = 53
    static let depth = 64
    static let promptDepth = 3
    static let alertDepth = 5
    static let alertSubrole = "AXNotificationCenterAlert"

    static func application(_ bundle: String) -> NSRunningApplication? {
        NSRunningApplication.runningApplications(withBundleIdentifier: bundle).first
    }

    static func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
        var value: CFTypeRef?
        return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
    }

    static func text(_ element: AXUIElement, _ name: String) -> String {
        attribute(element, name) as? String ?? ""
    }

    static func elements(under element: AXUIElement, depth: Int) -> [AXUIElement] {
        guard depth > 0 else { return [element] }
        let children = attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? []
        return [element] + children.flatMap { elements(under: $0, depth: depth - 1) }
    }

    static func page(of application: NSRunningApplication) -> [AXUIElement] {
        let root = AXUIElementCreateApplication(application.processIdentifier)
        AXUIElementSetAttributeValue(root, "AXManualAccessibility" as CFString, kCFBooleanTrue)
        let windows = attribute(root, kAXWindowsAttribute) as? [AXUIElement] ?? []
        return windows.flatMap { elements(under: $0, depth: depth) }
    }

    static func shown(in application: NSRunningApplication) -> [String] {
        page(of: application).flatMap { [text($0, kAXValueAttribute), text($0, kAXTitleAttribute), text($0, kAXDescriptionAttribute)] }
    }

    static func button(_ title: String, in application: NSRunningApplication) -> AXUIElement? {
        page(of: application).first { element in
            let role = text(element, kAXRoleAttribute)
            let named = text(element, kAXTitleAttribute) == title || text(element, kAXDescriptionAttribute) == title
            return named && (role == kAXButtonRole || role == "AXLink")
        }
    }

    static func tries(_ seconds: String) -> Int? {
        Double(seconds).map { max(1, Int($0 / pause)) }
    }

    static func waitFor(_ wanted: [String], in application: NSRunningApplication, tries: Int = attempts) -> Bool {
        for _ in 0..<tries {
            let texts = shown(in: application)
            if wanted.allSatisfy({ texts.contains($0) }) { return true }
            RunLoop.current.run(until: Date().addingTimeInterval(pause))
        }
        let texts = shown(in: application).filter { !$0.isEmpty }
        FileHandle.standardError.write(Data("page-checks probe: missing \(wanted) in \(texts)\n".utf8))
        return false
    }

    static func firstShown(_ wanted: [String], in application: NSRunningApplication, tries: Int) -> String? {
        for _ in 0..<tries {
            let texts = shown(in: application)
            if let found = wanted.first(where: { texts.contains($0) }) { return found }
            RunLoop.current.run(until: Date().addingTimeInterval(pause))
        }
        return nil
    }

    static func verdict(_ arguments: [String], in application: NSRunningApplication) -> Bool {
        guard arguments.count >= 6, let limit = tries(arguments[3]) else { return false }
        let passed = arguments[4]
        let found = firstShown([passed, arguments[5]], in: application, tries: limit)
        let prefixes = Array(arguments.dropFirst(6))
        var printed = Set<String>()
        for text in shown(in: application) where prefixes.contains(where: { text.hasPrefix($0) }) {
            if printed.insert(text).inserted { print(text) }
        }
        if found == nil { FileHandle.standardError.write(Data("page-checks probe: neither \(passed) nor \(arguments[5]) appeared\n".utf8)) }
        return found == passed
    }

    static func center(of element: AXUIElement) -> CGPoint? {
        guard let position = attribute(element, kAXPositionAttribute), let size = attribute(element, kAXSizeAttribute),
              CFGetTypeID(position) == AXValueGetTypeID(), CFGetTypeID(size) == AXValueGetTypeID() else { return nil }
        var origin = CGPoint.zero
        var extent = CGSize.zero
        guard AXValueGetValue(unsafeDowncast(position, to: AXValue.self), .cgPoint, &origin),
              AXValueGetValue(unsafeDowncast(size, to: AXValue.self), .cgSize, &extent) else { return nil }
        return CGPoint(x: origin.x + extent.width / 2, y: origin.y + extent.height / 2)
    }

    static func click(_ title: String, in application: NSRunningApplication) -> Bool {
        for _ in 0..<attempts {
            if let found = button(title, in: application), let point = center(of: found) {
                _ = application.activate()
                RunLoop.current.run(until: Date().addingTimeInterval(pause))
                for type in [CGEventType.mouseMoved, .leftMouseDown, .leftMouseUp] {
                    CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .left)?.post(tap: .cghidEventTap)
                }
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(pause))
        }
        FileHandle.standardError.write(Data("page-checks probe: no button \(title) to click\n".utf8))
        return false
    }

    static func escape(_ arguments: [String], in application: NSRunningApplication) -> Bool {
        guard arguments.count >= 5, let seconds = Double(arguments[3]) else { return false }
        let wanted = Array(arguments.dropFirst(4))
        for _ in 0..<max(1, Int(seconds / escapePause)) {
            if firstShown(wanted, in: application, tries: 1) != nil { return true }
            _ = application.activate()
            for down in [true, false] {
                CGEvent(keyboardEventSource: nil, virtualKey: escapeKey, keyDown: down)?.post(tap: .cghidEventTap)
            }
            RunLoop.current.run(until: Date().addingTimeInterval(escapePause))
        }
        FileHandle.standardError.write(Data("page-checks probe: none of \(wanted) after pressing Escape\n".utf8))
        return false
    }

    static func press(_ title: String, in application: NSRunningApplication) -> Bool {
        for _ in 0..<attempts {
            if let found = button(title, in: application) {
                return AXUIElementPerformAction(found, kAXPressAction as CFString) == .success
            }
            RunLoop.current.run(until: Date().addingTimeInterval(pause))
        }
        FileHandle.standardError.write(Data("page-checks probe: no button \(title)\n".utf8))
        return false
    }

    static func windowButton(_ titles: [String], in application: NSRunningApplication) -> AXUIElement? {
        let root = AXUIElementCreateApplication(application.processIdentifier)
        let windows = attribute(root, kAXWindowsAttribute) as? [AXUIElement] ?? []
        return windows.flatMap { elements(under: $0, depth: promptDepth) }.first { element in
            text(element, kAXRoleAttribute) == kAXButtonRole && titles.contains(text(element, kAXTitleAttribute))
        }
    }

    // macOS shows a notification request as a Notification Center alert whose Allow and Don't Allow are
    // named actions ("Name:Allow\nTarget:...") on the alert group, not buttons.
    static func alertAction(_ titles: [String], in application: NSRunningApplication) -> (AXUIElement, String)? {
        let root = AXUIElementCreateApplication(application.processIdentifier)
        let windows = attribute(root, kAXWindowsAttribute) as? [AXUIElement] ?? []
        for element in windows.flatMap({ elements(under: $0, depth: alertDepth) }) where text(element, kAXSubroleAttribute) == alertSubrole {
            var names: CFArray?
            AXUIElementCopyActionNames(element, &names)
            if let name = (names as? [String] ?? []).first(where: { name in titles.contains { name.hasPrefix("Name:\($0)\n") } }) {
                return (element, name)
            }
        }
        return nil
    }

    static func allow(_ titles: [String]) -> Bool {
        for _ in 0..<attempts {
            for candidate in NSWorkspace.shared.runningApplications {
                let owner = candidate.bundleIdentifier ?? String(candidate.processIdentifier)
                if let (alert, action) = alertAction(titles, in: candidate) {
                    print("pressed \(action.split(separator: "\n")[0]) in \(owner)")
                    return AXUIElementPerformAction(alert, action as CFString) == .success
                }
                guard let found = windowButton(titles, in: candidate) else { continue }
                print("pressed \(text(found, kAXTitleAttribute)) in \(owner)")
                return AXUIElementPerformAction(found, kAXPressAction as CFString) == .success
            }
            RunLoop.current.run(until: Date().addingTimeInterval(pause))
        }
        FileHandle.standardError.write(Data("page-checks probe: no window button named \(titles)\n".utf8))
        return false
    }

    static func quit(_ application: NSRunningApplication) -> Bool {
        guard application.terminate() else { return false }
        for _ in 0..<attempts where !application.isTerminated {
            RunLoop.current.run(until: Date().addingTimeInterval(pause))
        }
        return application.isTerminated
    }

    static func perform(_ arguments: [String], on application: NSRunningApplication) -> Bool {
        switch arguments[2] {
        case "wait":
            return waitFor(Array(arguments.dropFirst(3)), in: application)
        case "within":
            guard arguments.count >= 5, let limit = tries(arguments[3]) else { return false }
            return waitFor(Array(arguments.dropFirst(4)), in: application, tries: limit)
        case "press":
            return arguments.count == 4 && press(arguments[3], in: application)
        case "click":
            return arguments.count == 4 && click(arguments[3], in: application)
        case "escape":
            return escape(arguments, in: application)
        case "verdict":
            return verdict(arguments, in: application)
        case "quit":
            return quit(application)
        default:
            return false
        }
    }

    static func run() {
        let arguments = CommandLine.arguments
        guard AXIsProcessTrusted() else {
            FileHandle.standardError.write(Data("page-checks probe: this process needs Accessibility permission\n".utf8))
            exit(1)
        }
        if arguments.count >= 4, arguments[2] == "allow" {
            exit(allow(Array(arguments.dropFirst(3))) ? 0 : 1)
        }
        guard arguments.count >= 3, let application = application(arguments[1]) else {
            FileHandle.standardError.write(Data((usage + "\n").utf8))
            exit(2)
        }
        exit(perform(arguments, on: application) ? 0 : 1)
    }
}

PageChecksProbe.run()
