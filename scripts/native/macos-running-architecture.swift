import AppKit
import Foundation

enum RunningArchitecture {
    static let names: [Int: String] = [NSBundleExecutableArchitectureARM64: "arm64", NSBundleExecutableArchitectureX86_64: "x86_64"]
    static let attempts = 120
    static let pause: TimeInterval = 0.25

    static func application(_ bundle: String) -> NSRunningApplication? {
        for _ in 0..<attempts {
            if let found = NSRunningApplication.runningApplications(withBundleIdentifier: bundle).first { return found }
            RunLoop.current.run(until: Date().addingTimeInterval(pause))
        }
        return nil
    }

    static func run() {
        guard CommandLine.arguments.count == 2, let application = application(CommandLine.arguments[1]) else {
            FileHandle.standardError.write(Data("running architecture: no running app with that bundle id\n".utf8))
            exit(1)
        }
        print("\(application.processIdentifier) \(names[application.executableArchitecture] ?? String(application.executableArchitecture))")
    }
}

RunningArchitecture.run()
