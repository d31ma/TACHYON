@Service
class CounterService {
  int count = 0;
  @SessionState
  int visits = 0;
  @LocalState
  String theme = 'light';

  void increment() {
    count += 1;
  }
}
