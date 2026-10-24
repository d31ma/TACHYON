import AppKit
import ApplicationServices
import Foundation

enum ApplicationStateControl {
    static let usage = "usage: application-state-macos-control <bundle-id> activate|deactivate|hide|unhide|minimize|restore"
    static let otherApplication = "com.apple.finder"
    static let settle: TimeInterval = 0.5

    static func application(_ bundle: String) -> NSRunningApplication? {
        NSRunningApplication.runningApplications(withBundleIdentifier: bundle).first
    }

    static func firstWindow(of application: NSRunningApplication) -> AXUIElement? {
        let element = AXUIElementCreateApplication(application.processIdentifier)
        var windows: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element, kAXWindowsAttribute as CFString, &windows) == .success else {
            return nil
        }
        return (windows as? [AXUIElement])?.first
    }

    static func setMinimized(_ minimized: Bool, of application: NSRunningApplication) -> Bool {
        guard AXIsProcessTrusted(), let window = firstWindow(of: application) else { return false }
        return set(kAXMinimizedAttribute, minimized, on: window)
    }

    static func setHidden(_ hidden: Bool, of application: NSRunningApplication) -> Bool {
        guard AXIsProcessTrusted() else { return false }
        return set(kAXHiddenAttribute, hidden, on: AXUIElementCreateApplication(application.processIdentifier))
    }

    static func set(_ attribute: String, _ enabled: Bool, on element: AXUIElement) -> Bool {
        let value = enabled ? kCFBooleanTrue : kCFBooleanFalse
        return AXUIElementSetAttributeValue(element, attribute as CFString, value!) == .success
    }

    static func perform(_ action: String, on application: NSRunningApplication) -> Bool {
        switch action {
        case "activate":
            return application.activate()
        case "deactivate":
            return self.application(otherApplication)?.activate() ?? false
        case "hide":
            return setHidden(true, of: application)
        case "unhide":
            return setHidden(false, of: application)
        case "minimize":
            return setMinimized(true, of: application)
        case "restore":
            return setMinimized(false, of: application)
        default:
            return false
        }
    }

    static func run() {
        let arguments = CommandLine.arguments
        guard arguments.count == 3, let application = application(arguments[1]) else {
            FileHandle.standardError.write(Data((usage + "\n").utf8))
            exit(2)
        }
        guard perform(arguments[2], on: application) else {
            FileHandle.standardError.write(Data("application-state control: \(arguments[2]) failed; hide, unhide, minimize and restore need Accessibility permission\n".utf8))
            exit(1)
        }
        RunLoop.current.run(until: Date().addingTimeInterval(settle))
    }
}

ApplicationStateControl.run()
