@Service
class BasketService {
  int count = 0;
  String label = '';
  String _selected = '';
  List<Map<String, String>> tabs = [
    {'id': 'a', 'name': 'A'},
  ];

  void increment() => count += 1;

  void select(String id) => _selected = id;
}
