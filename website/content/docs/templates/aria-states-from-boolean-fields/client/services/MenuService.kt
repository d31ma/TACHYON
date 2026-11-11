@Service
class MenuService {
  var expanded: Boolean = false

  fun toggle() { expanded = !expanded }
}
