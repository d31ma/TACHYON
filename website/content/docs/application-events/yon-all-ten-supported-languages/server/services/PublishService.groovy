import models.ProbeReady

@Service
class PublishService {
  @Needs
  ApplicationEventPublisher applicationEventPublisher

  @OnStart
  void initialize() { applicationEventPublisher.publishEvent(new ProbeReady(7)) }
}
