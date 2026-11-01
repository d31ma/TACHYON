import AppKit
import Foundation
import Vision

@main
struct ScreenshotText {
    static func entry(_ text: String, _ box: CGRect) -> [String: Any] {
        ["text": text, "x": box.midX, "y": 1 - box.midY]
    }

    static func words(_ candidate: VNRecognizedText) -> [[String: Any]] {
        var found: [[String: Any]] = []
        let text = candidate.string
        text.enumerateSubstrings(in: text.startIndex..<text.endIndex, options: .byWords) { word, range, _, _ in
            guard let word, let box = try? candidate.boundingBox(for: range)?.boundingBox else { return }
            found.append(entry(word, box))
        }
        return found.count > 1 ? found : []
    }

    static func main() throws {
        let arguments = CommandLine.arguments.filter { $0 != "--words" }
        let wantWords = CommandLine.arguments.contains("--words")
        guard arguments.count == 2, let image = NSImage(contentsOfFile: arguments[1]),
              let picture = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
            FileHandle.standardError.write(Data("usage: screenshot-text [--words] <png>\n".utf8))
            exit(2)
        }
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = false
        try VNImageRequestHandler(cgImage: picture).perform([request])
        let lines: [[String: Any]] = (request.results ?? []).flatMap { observation -> [[String: Any]] in
            guard let candidate = observation.topCandidates(1).first else { return [] }
            return [entry(candidate.string, observation.boundingBox)] + (wantWords ? words(candidate) : [])
        }
        FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: lines))
    }
}
