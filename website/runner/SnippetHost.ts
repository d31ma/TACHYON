import type { Server } from 'bun'
import { ExampleCatalog } from './ExampleCatalog.ts'
import { ExampleServers } from './ExampleServers.ts'
import { HostRefusal } from './HostRefusal.ts'
import { RateLimiter } from './RateLimiter.ts'
import { RunForwarder } from './RunForwarder.ts'
import { RunQueue } from './RunQueue.ts'
import { RunRequestReader } from './RunRequestReader.ts'
import type { RunnerSettings } from './RunnerSettings.ts'
import type { RunnableExample } from './models/RunnableExample.ts'
import type { RunRequest } from './models/RunRequest.ts'
import type { RunResult } from './models/RunResult.ts'

export class SnippetHost {
  static readonly LOOPBACK = '127.0.0.1'
  static readonly CLIENT_HEADER = 'x-snippet-client'
  static readonly CLIENT = /^[0-9A-Fa-f:.]{2,45}$/
  static readonly RUN_PATH = /^\/runs\/([^/]+)$/
  static readonly HEALTH_PATH = '/health'
  static readonly LOOPBACK_PEERS = new Set(['127.0.0.1', '::1', '::ffff:127.0.0.1'])
  static readonly BUSY_RETRY_SECONDS = 5
  static readonly OK = 200
  static readonly BAD_REQUEST = 400
  static readonly FORBIDDEN = 403
  static readonly NOT_FOUND = 404
  static readonly TOO_MANY = 429
  static readonly UNAVAILABLE = 503

  private readonly catalog: ExampleCatalog
  private readonly servers: ExampleServers
  private readonly limiter: RateLimiter
  private readonly queue: RunQueue
  private readonly reader: RunRequestReader
  private readonly forwarder: RunForwarder
  private readonly server: Server<undefined>

  constructor(settings: RunnerSettings) {
    this.catalog = new ExampleCatalog(settings.projectsFolder)
    this.servers = new ExampleServers(settings)
    this.limiter = new RateLimiter(settings.runsPerMinute, settings.runsPerDay)
    this.queue = new RunQueue(settings.maxActiveRuns, settings.maxWaitingRuns)
    this.reader = new RunRequestReader(settings.maxBodyBytes)
    this.forwarder = new RunForwarder(settings.runMilliseconds, settings.maxAnswerBytes)
    this.server = Bun.serve({ hostname: SnippetHost.LOOPBACK, port: settings.port, fetch: (request, server) => this.handle(request, server) })
  }

  url(): string {
    return `http://${SnippetHost.LOOPBACK}:${this.server.port}`
  }

  runningServers(): number {
    return this.servers.count()
  }

  async stop(): Promise<void> {
    this.server.stop(true)
    await this.servers.stopAll()
  }

  private async handle(request: Request, server: Server<undefined>): Promise<Response> {
    try {
      this.requireLoopbackPeer(server.requestIP(request)?.address)
      return await this.route(request)
    } catch (error) {
      return this.refusalAnswer(error)
    }
  }

  private requireLoopbackPeer(address: string | undefined): void {
    if (SnippetHost.LOOPBACK_PEERS.has(address ?? '')) return
    throw new HostRefusal(SnippetHost.FORBIDDEN, { code: 'not-local', message: 'Only the website server on this machine may call the snippet host.' })
  }

  private async route(request: Request): Promise<Response> {
    const url = new URL(request.url)
    if (request.method === 'GET' && url.pathname === SnippetHost.HEALTH_PATH) return this.answer(SnippetHost.OK, { examples: this.catalog.size() })
    const runId = this.runIdOf(request.method, url.pathname)
    return this.answer(SnippetHost.OK, await this.run(runId, request))
  }

  private runIdOf(method: string, pathname: string): string {
    const match = pathname.match(SnippetHost.RUN_PATH)
    if (method === 'POST' && match) return match[1]
    throw new HostRefusal(SnippetHost.NOT_FOUND, { code: 'not-found', message: 'Use POST /runs/<run id>.' })
  }

  private async run(runId: string, request: Request): Promise<RunResult> {
    const client = this.clientOf(request.headers.get(SnippetHost.CLIENT_HEADER))
    const example = this.exampleOf(runId)
    const runRequest = await this.reader.read(request)
    this.requireRoute(example, runRequest)
    this.requireAllowance(client)
    return await this.runInTurn(example, runRequest)
  }

  private clientOf(header: string | null): string {
    if (header !== null && SnippetHost.CLIENT.test(header)) return header
    throw new HostRefusal(SnippetHost.BAD_REQUEST, { code: 'no-client', message: `The ${SnippetHost.CLIENT_HEADER} header must hold the client address.` })
  }

  private exampleOf(runId: string): RunnableExample {
    const example = this.catalog.find(runId)
    if (example !== undefined) return example
    throw new HostRefusal(SnippetHost.NOT_FOUND, { code: 'unknown-example', message: 'This example cannot run here.' })
  }

  private requireRoute(example: RunnableExample, request: RunRequest): void {
    if (this.catalog.allows(example, request.method, request.path)) return
    throw new HostRefusal(SnippetHost.NOT_FOUND, { code: 'unknown-route', message: `The example has no ${request.method} ${request.path} route.` })
  }

  private requireAllowance(client: string): void {
    const decision = this.limiter.take(client, Date.now())
    if (decision.allowed) return
    throw new HostRefusal(SnippetHost.TOO_MANY, { code: 'rate-limited', message: 'Too many runs. Try again later.', retryAfterSeconds: decision.retryAfterSeconds })
  }

  private async runInTurn(example: RunnableExample, request: RunRequest): Promise<RunResult> {
    this.requireTurn(await this.queue.enter())
    try {
      return this.requireResult(await this.servers.use(example.runId, this.catalog.projectFolder(example), server => this.forwarder.forward(server, request)))
    } finally {
      this.queue.leave()
    }
  }

  private requireTurn(entered: boolean): void {
    if (entered) return
    throw this.busy()
  }

  private requireResult(result: RunResult | undefined): RunResult {
    if (result !== undefined) return result
    throw this.busy()
  }

  private busy(): HostRefusal {
    return new HostRefusal(SnippetHost.UNAVAILABLE, { code: 'busy', message: 'The runner is busy. Try again soon.', retryAfterSeconds: SnippetHost.BUSY_RETRY_SECONDS })
  }

  private refusalAnswer(error: unknown): Response {
    if (error instanceof HostRefusal) return this.answer(error.status, error.problem, error.problem.retryAfterSeconds)
    console.error(`snippet host: ${error instanceof Error ? error.message : String(error)}`)
    return this.answer(SnippetHost.UNAVAILABLE, { code: 'unavailable', message: 'The example could not run.' }, SnippetHost.BUSY_RETRY_SECONDS)
  }

  private answer(status: number, value: object, retryAfterSeconds?: number): Response {
    const headers: Record<string, string> = { 'content-type': 'application/json', 'cache-control': 'no-store' }
    if (retryAfterSeconds !== undefined) headers['retry-after'] = String(retryAfterSeconds)
    return new Response(JSON.stringify(value), { status, headers })
  }
}
