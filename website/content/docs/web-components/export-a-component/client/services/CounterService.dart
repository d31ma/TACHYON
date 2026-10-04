import 'models/CountChanged.dart';

@Service
class CounterService {
  @Needs
  final ApplicationEventPublisher applicationEventPublisher;
  int count = 0;

  void add() {
    count += 1;
    applicationEventPublisher.publishEvent(CountChanged(count));
  }
}
