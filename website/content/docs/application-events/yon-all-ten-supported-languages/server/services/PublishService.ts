import type { ProbeReady } from '/models/ProbeReady.ts';

@Service
export default class PublishService {
  @Needs
  applicationEventPublisher!: ApplicationEventPublisher;

  @OnStart
  initialize() {
    const ready: ProbeReady = { value: 7 };
    this.applicationEventPublisher.publishEvent(ready);
  }
}
