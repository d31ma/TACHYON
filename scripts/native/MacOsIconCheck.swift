import AppKit
import Foundation

@main
@MainActor
private struct MacOsIconCheck {
    private static let edge = 64

    static func main() throws {
        let arguments = CommandLine.arguments
        guard arguments.count == 3 else { throw failure("Usage: MacOsIconCheck <bundle id> <app path>") }
        let running = NSRunningApplication.runningApplications(withBundleIdentifier: arguments[1]).first
        guard let dockIcon = running?.icon else { throw failure("No running app has the bundle id \(arguments[1])") }
        let fileIcon = NSWorkspace.shared.icon(forFile: arguments[2])
        let report = ["running": try center(dockIcon), "file": try center(fileIcon)]
        FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: report))
    }

    private static func center(_ image: NSImage) throws -> [Int] {
        var frame = NSRect(x: 0, y: 0, width: edge, height: edge)
        guard let picture = image.cgImage(forProposedRect: &frame, context: nil, hints: nil) else {
            throw failure("The icon has no bitmap")
        }
        guard let space = CGColorSpace(name: CGColorSpace.sRGB),
              let context = CGContext(data: nil, width: edge, height: edge, bitsPerComponent: 8, bytesPerRow: edge * 4,
                                      space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
            throw failure("No sRGB bitmap context")
        }
        context.draw(picture, in: CGRect(x: 0, y: 0, width: edge, height: edge))
        guard let pixels = context.data?.assumingMemoryBound(to: UInt8.self) else { throw failure("The bitmap has no pixels") }
        let offset = (edge / 2 * edge + edge / 2) * 4
        return [Int(pixels[offset]), Int(pixels[offset + 1]), Int(pixels[offset + 2])]
    }

    private static func failure(_ message: String) -> NSError {
        NSError(domain: "MacOsIconCheck", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}
