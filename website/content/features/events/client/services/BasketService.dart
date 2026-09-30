import 'models/Tab.dart';

@Service
class BasketService {
  int count = 0;
  String label = '';
  String _selected = '';
  List<Tab> tabs = [const Tab('a', 'A')];

  void increment() => count += 1;

  void select(String id) => _selected = id;
}
