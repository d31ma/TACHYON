@Service
class BasketService {
  int count = 0;

  int doubled() { return count * 2; }

  @OnStart
  void initialize() { count = 6; }
}
