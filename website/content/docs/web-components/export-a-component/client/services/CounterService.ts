import type { CountChanged } from '/models/CountChanged.ts';

@Service
@NeedsAll
export default class CounterService {
  start!: string;
  applicationEventPublisher!: ApplicationEventPublisher;
  count = 0;

  @OnStart
  initialize(): void {
    this.count = Number(this.start);
  }

  add(): void {
    this.count += 1;
    const changed: CountChanged = { count: this.count };
    this.applicationEventPublisher.publishEvent(changed);
  }
}
