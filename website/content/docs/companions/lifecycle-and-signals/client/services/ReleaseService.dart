import 'models/ReleaseChanged.dart';

@Service
class ReleaseService {
  @Needs
  final ApplicationEventPublisher applicationEventPublisher;
  String release = '';

  void announce() {
    applicationEventPublisher.publishEvent(const ReleaseChanged('TACHYON'), true);
  }

  @OnEvent({"replay":true})
  void receive(ReleaseChanged changed) {
    release = changed.release;
  }
}
