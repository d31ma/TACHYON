@Service
class OrdersService {
  late BehaviorContext behaviors;
  static const _firstWorkingAttempt = 3;
  int attempts = 0;

  @OnStart
  void open() {
    behaviors = Behaviors.create(OrdersSetup, partition: 'orders', signal: BehaviorSignal());
  }

  @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
  Future<Object?> submit(String id) async {
    attempts += 1;
    _failFirstTwoAttempts();
    return _reportOf(id);
  }

  @MaxAtOnce({"limit":2,"policy":"block"})
  Future<Object?> report(String id) async {
    return _reportOf(id);
  }

  Map<String, Object?> _reportOf(String id) => {'id': id, 'attempts': attempts};

  void _failFirstTwoAttempts() {
    if (attempts < _firstWorkingAttempt) throw StateError('the checkout API is unavailable');
  }
}
