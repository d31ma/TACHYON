import AppKit
import Foundation
import Vision

@main
struct ScreenshotText {
    static let whole = CGRect(x: 0, y: 0, width: 1, height: 1)

    static func entry(_ text: String, _ box: CGRect, _ area: CGRect) -> [String: Any] {
        ["text": text, "x": area.minX + box.midX * area.width, "y": area.minY + (1 - box.midY) * area.height]
    }

    static func words(_ candidate: VNRecognizedText, _ area: CGRect) -> [[String: Any]] {
        var found: [[String: Any]] = []
        let text = candidate.string
        text.enumerateSubstrings(in: text.startIndex..<text.endIndex, options: .byWords) { word, range, _, _ in
            guard let word, let box = try? candidate.boundingBox(for: range)?.boundingBox else { return }
            found.append(entry(word, box, area))
        }
        return found.count > 1 ? found : []
    }

    static func region(_ arguments: [String]) -> CGRect? {
        guard let index = arguments.firstIndex(of: "--region") else { return whole }
        let parts = arguments.dropFirst(index + 1).first?.split(separator: ",").compactMap { Double($0) } ?? []
        guard parts.count == 4 else { return nil }
        let area = CGRect(x: parts[0], y: parts[1], width: parts[2], height: parts[3]).intersection(whole)
        return area.isEmpty ? nil : area
    }

    static func files(_ arguments: [String]) -> [String] {
        let value = arguments.firstIndex(of: "--region").map { $0 + 1 }
        return arguments.indices.filter { $0 != value && !arguments[$0].hasPrefix("--") }.map { arguments[$0] }
    }

    static func crop(_ picture: CGImage, _ area: CGRect) -> CGImage? {
        let width = CGFloat(picture.width), height = CGFloat(picture.height)
        let pixels = CGRect(x: area.minX * width, y: area.minY * height, width: area.width * width, height: area.height * height)
        return picture.cropping(to: pixels.integral)
    }

    static func main() throws {
        let arguments = Array(CommandLine.arguments.dropFirst())
        let wantWords = arguments.contains("--words")
        let files = files(arguments)
        guard files.count == 1, let area = region(arguments), let image = NSImage(contentsOfFile: files[0]),
              let picture = image.cgImage(forProposedRect: nil, context: nil, hints: nil),
              let part = crop(picture, area) else {
            FileHandle.standardError.write(Data("usage: screenshot-text [--words] [--region x,y,width,height] <png>\n".utf8))
            exit(2)
        }
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = false
        try VNImageRequestHandler(cgImage: part).perform([request])
        let lines: [[String: Any]] = (request.results ?? []).flatMap { observation -> [[String: Any]] in
            guard let candidate = observation.topCandidates(1).first else { return [] }
            return [entry(candidate.string, observation.boundingBox, area)] + (wantWords ? words(candidate, area) : [])
        }
        FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: lines))
    }
}
