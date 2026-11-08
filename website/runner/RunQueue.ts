export class RunQueue {
  private active = 0
  private readonly waiting: (() => void)[] = []

  constructor(private readonly maxActive: number, private readonly maxWaiting: number) {}

  enter(): Promise<boolean> {
    return this.admit()
  }

  leave(): void {
    this.handOver(this.waiting.shift())
  }

  private admit(): Promise<boolean> {
    if (this.active < this.maxActive) return this.start()
    return this.waitForTurn()
  }

  private start(): Promise<boolean> {
    this.active += 1
    return Promise.resolve(true)
  }

  private waitForTurn(): Promise<boolean> {
    if (this.waiting.length >= this.maxWaiting) return Promise.resolve(false)
    return new Promise(resolve => this.waiting.push(() => resolve(true)))
  }

  private handOver(next: (() => void) | undefined): void {
    if (next === undefined) {
      this.active -= 1
      return
    }
    next()
  }
}
