@Service
class TallyService {
  late CounterService counter;
  int total = 0;

  void bump() {
    total = counter.increment();
  }
}
