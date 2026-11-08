import type { RecordedExchange } from '/models/RecordedExchange.ts'
import type { RecordedRun } from '/models/RecordedRun.ts'
import type { TaskScheduler } from '/models/TaskScheduler.ts'

@Service
@Lifetime({"value":"prototype"})
export default class SnippetRunService {
  private static readonly RUN_SUFFIX = '.json'
  private static readonly JSON_TYPE = 'json'
  private static readonly JSON_INDENT = 2
  private static readonly RUN_LABEL = 'Run'
  private static readonly HIDE_LABEL = 'Hide output'
  private static readonly LOADING_MESSAGE = 'Loading the recorded output'
  private static readonly FAILED_MESSAGE = 'No recorded output in this build.'
  @Needs
  recordedRunClient!: RecordedRunClient
  @Needs
  taskScheduler!: TaskScheduler
  @Needs
  lifecycle!: AbortSignal
  private shownRun = ''
  private messages = new Map<string, string>()
  private runs = new Map<string, RecordedExchange[]>()

  label(run: string): string {
    return this.labelFor(this.isShown(run))
  }

  status(run: string): string {
    return this.messageOf(run)
  }

  isShown(run: string): boolean {
    return this.shownWithExchanges(run)
  }

  exchanges(run: string): RecordedExchange[] {
    return this.loadedExchanges(run)
  }

  requestText(exchange: RecordedExchange): string {
    const lines = [...this.headerLines(exchange.request.headers), this.readable(this.bodyText(exchange.request.body))]
    return lines.join('\n').trim()
  }

  responseText(exchange: RecordedExchange): string {
    const body = exchange.response.body
    return this.readableWhenJson(exchange.response.contentType, body, this.readable(body))
  }

  async toggle(_event: unknown, run: string): Promise<void> {
    const hiding = this.isShown(run)
    this.shownRun = ''
    await this.loadUnlessHiding(hiding, run)
    await this.renderWhileMounted()
  }

  private labelFor(shown: boolean): string {
    if (shown) return SnippetRunService.HIDE_LABEL
    return SnippetRunService.RUN_LABEL
  }

  private messageOf(run: string): string {
    return this.messages.get(run) ?? ''
  }

  private shownWithExchanges(run: string): boolean {
    return this.shownRun === run && this.runs.has(run)
  }

  private loadedExchanges(run: string): RecordedExchange[] {
    return this.runs.get(run) ?? []
  }

  private headerLines(headers: Record<string, string>): string[] {
    return Object.entries(headers).map(([name, value]) => `${name}: ${value}`)
  }

  private bodyText(body: string | undefined): string {
    return body ?? ''
  }

  private readable(body: string): string {
    try {
      return JSON.stringify(JSON.parse(body), null, SnippetRunService.JSON_INDENT)
    } catch {
      return body
    }
  }

  private readableWhenJson(contentType: string, body: string, readableBody: string): string {
    if (contentType.includes(SnippetRunService.JSON_TYPE)) return readableBody
    return body
  }

  private async loadUnlessHiding(hiding: boolean, run: string): Promise<void> {
    if (hiding) return
    await this.load(run)
  }

  private async load(run: string): Promise<void> {
    this.messages.set(run, SnippetRunService.LOADING_MESSAGE)
    await this.renderWhileMounted()
    const recorded = await this.recordedRunClient.run(`${run}${SnippetRunService.RUN_SUFFIX}`).catch(() => undefined)
    this.remember(run, recorded)
  }

  private remember(run: string, recorded: RecordedRun | void | undefined): void {
    if (!recorded) {
      this.messages.set(run, SnippetRunService.FAILED_MESSAGE)
      return
    }
    this.messages.delete(run)
    this.runs.set(run, recorded.exchanges)
    this.shownRun = run
  }

  private async renderWhileMounted(): Promise<void> {
    if (this.lifecycle.aborted) return
    await this.taskScheduler.render()
  }
}
