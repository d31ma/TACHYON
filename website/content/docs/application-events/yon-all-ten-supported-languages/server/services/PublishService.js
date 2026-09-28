@Service
class PublishService {
  /** @type {ApplicationEventPublisher} */
  @Needs
  applicationEventPublisher;

  @OnStart
  initialize() {
    /** @type {ProbeReady} */
    const ready = { value: 7 };
    this.applicationEventPublisher.publishEvent(ready);
  }
}
