@Service
class MenuService {
  bool expanded = false;

  void toggle() {
    expanded = !expanded;
  }

  void close() {
    expanded = false;
  }
}
