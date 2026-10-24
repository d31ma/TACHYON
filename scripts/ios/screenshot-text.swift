import AppKit
import Foundation
import Vision

@main
struct ScreenshotText {
    static func main() throws {
        let arguments = CommandLine.arguments
        guard arguments.count == 2, let image = NSImage(contentsOfFile: arguments[1]),
              let picture = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
            FileHandle.standardError.write(Data("usage: screenshot-text <png>\n".utf8))
            exit(2)
        }
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = false
        try VNImageRequestHandler(cgImage: picture).perform([request])
        let lines: [[String: Any]] = (request.results ?? []).compactMap { observation in
            guard let text = observation.topCandidates(1).first?.string else { return nil }
            let box = observation.boundingBox
            return ["text": text, "x": box.midX, "y": 1 - box.midY]
        }
        FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: lines))
    }
}
