@Service
final class FeatureFilterService {
  private static let space: Character = " "

  var query: String = ""

  func find(_ text: String) {
    query = text
  }

  func shows(_ title: String, _ summary: String) -> Bool {
    return containsEveryWord(of: query.lowercased(), in: (title + String(Self.space) + summary).lowercased())
  }

  private func containsEveryWord(of wanted: String, in text: String) -> Bool {
    let letters = Array(text.utf8)
    return wanted.split(separator: Self.space).allSatisfy { word in appears(Array(word.utf8), in: letters) }
  }

  private func appears(_ word: [UInt8], in letters: [UInt8]) -> Bool {
    return letters.indices.contains { start in letters[start...].starts(with: word) }
  }
}
