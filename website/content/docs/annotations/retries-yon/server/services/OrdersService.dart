@Service
class OrdersService {
  late BehaviorContext behaviors;
  static const _firstWorkingAttempt = 3;
  int _attempts = 0;

  @OnStart
  void open() {
    behaviors = Behaviors.create(OrdersSetup, partition: 'orders', signal: BehaviorSignal());
  }

  @Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})
  Future<Object?> submit(String id) async {
    _attempts += 1;
    _failFirstTwoAttempts();
    return _reportOf(id);
  }

  @MaxAtOnce({"limit":2,"policy":"block"})
  Future<Object?> report(String id) async {
    return _reportOf(id);
  }

  Map<String, Object?> _reportOf(String id) => {'id': id, 'attempts': _attempts};

  void _failFirstTwoAttempts() {
    if (_attempts < _firstWorkingAttempt) throw StateError('the payment gateway is unavailable');
  }
}
