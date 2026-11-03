@Service
class TallyService {
  int count = 0;

  void add(int amount) {
    count += amount;
  }

  String summary(String unit) => '$count $unit';
}
