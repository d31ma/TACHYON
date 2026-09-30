import models.Tab

@Service
class BasketService {
  var count: Int = 0
  var label: String = ""
  private var selected: String = ""
  var tabs: List<Tab> = listOf(Tab("a", "A"))

  fun increment() {
    count += 1
  }

  fun select(id: String) {
    selected = id
  }
}
