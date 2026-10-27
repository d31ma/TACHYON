import type { LedgerEntry } from '/models/LedgerEntry.ts';

@Service
export default class TillService {
  applicationEventPublisher!: ApplicationEventPublisher;
  steps: string[] = [];

  @OnStart
  start(): void { this.steps = [...this.steps, 'start']; }

  @OnStop
  stop(): void { this.steps = [...this.steps, 'stop']; }

  post(amount: number): void {
    const entry: LedgerEntry = { amount };
    this.applicationEventPublisher.publishEvent(entry);
  }
}
