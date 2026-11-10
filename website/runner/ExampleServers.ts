import { ExampleServer } from './ExampleServer.ts'
import type { RunnerSettings } from './RunnerSettings.ts'

export class ExampleServers {
  static readonly CHECKS_PER_IDLE_PERIOD = 4

  private readonly servers = new Map<string, Promise<ExampleServer>>()
  private readonly timer: ReturnType<typeof setInterval>

  constructor(private readonly settings: RunnerSettings) {
    this.timer = setInterval(() => void this.stopIdle(Date.now()), settings.idleMilliseconds / ExampleServers.CHECKS_PER_IDLE_PERIOD)
  }

  async use<T>(runId: string, projectFolder: string, work: (server: ExampleServer) => Promise<T>): Promise<T | undefined> {
    const server = await this.serverFor(runId, projectFolder)
    return await this.runOn(server, work)
  }

  count(): number {
    return this.servers.size
  }

  async stopAll(): Promise<void> {
    clearInterval(this.timer)
    const all = [...this.servers.values()]
    this.servers.clear()
    await Promise.all(all.map(async started => (await started).stop()))
  }

  private async serverFor(runId: string, projectFolder: string): Promise<ExampleServer | undefined> {
    const running = this.servers.get(runId)
    if (running !== undefined) return await running
    return await this.startWithRoom(runId, projectFolder, await this.makeRoom())
  }

  private async startWithRoom(runId: string, projectFolder: string, room: boolean): Promise<ExampleServer | undefined> {
    if (!room) return undefined
    const starting = ExampleServer.start(projectFolder, this.settings)
    this.servers.set(runId, starting)
    return await starting.catch(error => this.forget(runId, error))
  }

  private forget(runId: string, error: unknown): never {
    this.servers.delete(runId)
    throw error
  }

  private async makeRoom(): Promise<boolean> {
    const settled = await this.settledServers()
    const idle = settled.filter(([, server]) => server.activeRuns === 0).sort(([, first], [, second]) => first.lastUsed - second.lastUsed)
    return await this.stopOldestWhenFull(idle)
  }

  private async stopOldestWhenFull(idle: [string, ExampleServer][]): Promise<boolean> {
    if (this.servers.size < this.settings.maxServers) return true
    const oldest = idle[0]
    if (oldest === undefined) return false
    await this.stopServer(oldest[0], oldest[1])
    return true
  }

  private async runOn<T>(server: ExampleServer | undefined, work: (server: ExampleServer) => Promise<T>): Promise<T | undefined> {
    if (server === undefined) return undefined
    server.activeRuns += 1
    server.runsStarted += 1
    server.lastUsed = Date.now()
    try {
      return await work(server)
    } finally {
      server.activeRuns -= 1
      server.lastUsed = Date.now()
    }
  }

  private async stopIdle(now: number): Promise<void> {
    const settled = await this.settledServers()
    const expired = settled.filter(([, server]) => server.activeRuns === 0 && now - server.lastUsed >= this.settings.idleMilliseconds)
    await Promise.all(expired.map(([runId, server]) => this.stopServer(runId, server)))
  }

  private async settledServers(): Promise<[string, ExampleServer][]> {
    const entries = [...this.servers.entries()]
    const settled = await Promise.all(entries.map(async ([runId, starting]) => [runId, await starting.catch(() => undefined)] as const))
    return settled.filter((entry): entry is [string, ExampleServer] => entry[1] !== undefined)
  }

  private async stopServer(runId: string, server: ExampleServer): Promise<void> {
    this.servers.delete(runId)
    await server.stop()
  }
}
