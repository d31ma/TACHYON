@Service
final class GroupFilterService {
  var group: String = ""

  func pick(_ name: String) {
    group = name
  }

  func shows(_ name: String) -> Bool {
    return isChosen(name)
  }

  private func isChosen(_ name: String) -> Bool {
    return group.isEmpty || group == name
  }
}
