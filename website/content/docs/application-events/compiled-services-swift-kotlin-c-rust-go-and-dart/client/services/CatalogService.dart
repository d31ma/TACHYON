import 'models/CatalogCount.dart';

@Service
class CatalogService {
  @Needs
  final ApplicationEventPublisher applicationEventPublisher;
  int count = 0;

  @OnStart
  void initialize() {
    _announce();
  }

  void _announce() {
    applicationEventPublisher.publishEvent(const CatalogCount(5));
  }

  @OnEvent
  void changed(CatalogCount value) {
    count = value.count;
  }

  @OnStop
  void cleanup() {
    count = 0;
  }
}
