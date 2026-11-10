import type { RateDecision } from './models/RateDecision.ts'

export class RateLimiter {
  static readonly MINUTE = 60_000
  static readonly DAY = 24 * 60 * RateLimiter.MINUTE
  static readonly MILLISECONDS_PER_SECOND = 1000

  private readonly minuteCounts = new Map<string, number[]>()
  private readonly dayCounts = new Map<string, number[]>()

  constructor(private readonly perMinute: number, private readonly perDay: number) {}

  take(client: string, now: number): RateDecision {
    const minute = this.recent(this.minuteCounts, client, now - RateLimiter.MINUTE)
    const day = this.recent(this.dayCounts, client, now - RateLimiter.DAY)
    const wait = Math.max(this.waitFor(minute, this.perMinute, RateLimiter.MINUTE, now), this.waitFor(day, this.perDay, RateLimiter.DAY, now))
    this.record(minute, day, wait, now)
    return { allowed: wait === 0, retryAfterSeconds: Math.ceil(wait / RateLimiter.MILLISECONDS_PER_SECOND) }
  }

  private recent(counts: Map<string, number[]>, client: string, since: number): number[] {
    const kept = (counts.get(client) ?? []).filter(time => time > since)
    counts.set(client, kept)
    return kept
  }

  private waitFor(times: number[], limit: number, window: number, now: number): number {
    if (times.length < limit) return 0
    return times[times.length - limit] + window - now
  }

  private record(minute: number[], day: number[], wait: number, now: number): void {
    if (wait > 0) return
    minute.push(now)
    day.push(now)
  }
}
