import models.ProbeReady

@Service
class PublishService {
  @Needs
  lateinit var applicationEventPublisher: ApplicationEventPublisher

  @OnStart
  fun initialize() { applicationEventPublisher.publishEvent(ProbeReady(7)) }
}
