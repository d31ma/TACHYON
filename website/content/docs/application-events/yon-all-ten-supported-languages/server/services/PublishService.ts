import type { ProbeReady } from '/models/ProbeReady';

@Service
class PublishService {
  @Needs
  applicationEventPublisher!: ApplicationEventPublisher;

  @OnStart
  initialize() {
    const ready: ProbeReady = { value: 7 };
    this.applicationEventPublisher.publishEvent(ready);
  }
}
