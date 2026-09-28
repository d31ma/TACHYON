import 'models/ProbeReady.dart';

@Service
class PublishService {
  @Needs
  final ApplicationEventPublisher applicationEventPublisher;

  @OnStart
  void initialize() {
    applicationEventPublisher.publishEvent(const ProbeReady(7));
  }
}
