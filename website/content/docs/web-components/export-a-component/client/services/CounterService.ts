import type { CountChanged } from '/models/CountChanged.ts';

@Service
export default class CounterService {
  applicationEventPublisher!: ApplicationEventPublisher;
  count = 0;

  add(): void {
    this.count += 1;
    const changed: CountChanged = { count: this.count };
    this.applicationEventPublisher.publishEvent(changed);
  }
}
