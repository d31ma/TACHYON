@Service
final class BasketService {
  var count: Int = 0
  var label: String = ""
  private var selected: String = ""
  var tabs: [Tab] = [Tab(id: "a", name: "A")]

  func increment() {
    count += 1
  }

  func select(_ id: String) {
    selected = id
  }
}
