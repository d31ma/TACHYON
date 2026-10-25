import AppKit
import ApplicationServices
import Foundation

enum PageChecksProbe {
    static let usage = "usage: page-checks-macos-probe <bundle-id> wait <text>... | press <title> | quit | - allow <title>..."
    static let attempts = 120
    static let pause: TimeInterval = 0.25
    static let depth = 64

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

    static func waitFor(_ wanted: [String], in application: NSRunningApplication) -> Bool {
        for _ in 0..<attempts {
            let texts = shown(in: application)
            if wanted.allSatisfy({ texts.contains($0) }) { return true }
            RunLoop.current.run(until: Date().addingTimeInterval(pause))
        }
        let texts = shown(in: application).filter { !$0.isEmpty }
        FileHandle.standardError.write(Data("page-checks probe: missing \(wanted) in \(texts)\n".utf8))
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
        return windows.flatMap { elements(under: $0, depth: depth) }.first { element in
            text(element, kAXRoleAttribute) == kAXButtonRole && titles.contains(text(element, kAXTitleAttribute))
        }
    }

    static func allow(_ titles: [String]) -> Bool {
        for _ in 0..<attempts {
            for candidate in NSWorkspace.shared.runningApplications {
                guard let found = windowButton(titles, in: candidate) else { continue }
                print("pressed \(text(found, kAXTitleAttribute)) in \(candidate.bundleIdentifier ?? String(candidate.processIdentifier))")
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
        case "press":
            return arguments.count == 4 && press(arguments[3], in: application)
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
