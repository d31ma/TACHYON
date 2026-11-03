import type { LedgerEntry } from '/models/LedgerEntry.ts';

@Service
export default class LedgerService {
  heard: string[] = [];

  @OnEvent
  @Order(5)
  late(entry: LedgerEntry): void { this.heard = [...this.heard, `late ${entry.amount}`]; }

  @OnEvent
  @Order(-5)
  early(entry: LedgerEntry): void { this.heard = [...this.heard, `early ${entry.amount}`]; }

  @OnEvent
  @InBackground
  later(entry: LedgerEntry): void { this.heard = [...this.heard, `background ${entry.amount}`]; }
}
