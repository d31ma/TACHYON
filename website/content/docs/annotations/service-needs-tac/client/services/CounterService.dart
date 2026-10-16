@Service
class CounterService {
  int count = 0;

  int increment() {
    count += 1;
    return count;
  }
}
