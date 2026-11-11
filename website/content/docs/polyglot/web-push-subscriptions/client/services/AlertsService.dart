@Service
class AlertsService {
  static const serverKey = 'BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U';
  @Needs()
  final Push push;
  String endpoint = '';
  String problem = '';

  void turnOn() => _keepProblem(() => push.subscribe(serverKey));

  void turnOff() => _keepProblem(() => push.unsubscribe());

  @OnEvent
  void subscribed(PushSubscribed subscribed) {
    endpoint = subscribed.endpoint;
  }

  @OnEvent
  void unsubscribed(PushUnsubscribed unsubscribed) {
    endpoint = '';
  }

  @OnEvent
  void failed(BrowserCallFailed failed) {
    problem = failed.message;
  }

  void _keepProblem(void Function() call) {
    try {
      call();
    } on StateError catch (refusal) {
      problem = refusal.message;
    }
  }
}
