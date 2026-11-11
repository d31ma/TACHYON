@Service
final class MenuService {
  var expanded: Bool = false

  func toggle() { expanded = !expanded }
}
