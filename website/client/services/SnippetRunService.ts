import type { LiveRunResponse } from '/models/LiveRunResponse.ts'
import type { LiveRunResult } from '/models/LiveRunResult.ts'
import type { RecordedExchange } from '/models/RecordedExchange.ts'
import type { RecordedRun } from '/models/RecordedRun.ts'
import type { RunDraft } from '/models/RunDraft.ts'
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
  private static readonly LIVE_MESSAGE = 'Running the example on the live runner'
  private static readonly OFFLINE_MESSAGE = 'The live runner did not answer. The output above is the recorded run.'
  private static readonly TIMEOUT_MESSAGE = 'The example took too long and was stopped.'
  private static readonly TRUNCATED_NOTE = '\n[The answer was cut at 64 KiB.]'
  private static readonly QUERY_MARK = '?'
  private static readonly OK_OUTCOME = 'ok'
  private static readonly OFFLINE_OUTCOME = 'offline'
  private static readonly BODY_METHODS = ['POST', 'PUT', 'PATCH']
  private static readonly EMPTY_DRAFT: RunDraft = { method: 'GET', path: '/', body: '' }
  @Needs
  recordedRunClient!: RecordedRunClient
  @Needs
  snippetRunClient!: SnippetRunClient
  @Needs
  taskScheduler!: TaskScheduler
  @Needs
  lifecycle!: AbortSignal
  private shownRun = ''
  private messages = new Map<string, string>()
  private runs = new Map<string, RecordedExchange[]>()
  private drafts = new Map<string, RunDraft>()
  private liveMessages = new Map<string, string>()
  private liveResults = new Map<string, LiveRunResult>()

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

  draftMethod(run: string): string {
    return this.draftOf(run).method
  }

  draftPath(run: string): string {
    return this.draftOf(run).path
  }

  draftBody(run: string): string {
    return this.draftOf(run).body
  }

  takesBody(run: string): boolean {
    return SnippetRunService.BODY_METHODS.includes(this.draftOf(run).method)
  }

  editPath(_event: unknown, run: string, path: string): void {
    this.drafts.set(run, { ...this.draftOf(run), path })
  }

  editBody(_event: unknown, run: string, body: string): void {
    this.drafts.set(run, { ...this.draftOf(run), body })
  }

  async runLive(_event: unknown, run: string): Promise<void> {
    this.liveMessages.set(run, SnippetRunService.LIVE_MESSAGE)
    this.liveResults.delete(run)
    await this.renderWhileMounted()
    const result = await this.snippetRunClient.run(run, this.liveRequest(this.draftOf(run))).catch(() => undefined)
    this.liveMessages.set(run, this.messageForOutcome(this.outcomeOf(result), this.durationOf(result)))
    this.rememberLive(run, result)
    await this.renderWhileMounted()
  }

  liveMessage(run: string): string {
    return this.textOf(this.liveMessages.get(run))
  }

  hasLiveAnswer(run: string): boolean {
    return this.liveResponseOf(run) !== undefined
  }

  liveStatus(run: string): string {
    return this.statusText(this.liveResponseOf(run))
  }

  liveContentType(run: string): string {
    return this.contentTypeOf(this.liveResponseOf(run))
  }

  liveBody(run: string): string {
    const response = this.liveResponseOf(run)
    const body = this.bodyText(response?.body)
    const readableBody = this.readableWhenJson(this.contentTypeOf(response), body, this.readable(body))
    return `${readableBody}${this.truncatedNote(response?.truncated)}`
  }

  private labelFor(shown: boolean): string {
    if (shown) return SnippetRunService.HIDE_LABEL
    return SnippetRunService.RUN_LABEL
  }

  private messageOf(run: string): string {
    return this.textOf(this.messages.get(run))
  }

  private textOf(text: string | undefined): string {
    return text ?? ''
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

  private liveResponseOf(run: string): LiveRunResponse | undefined {
    return this.liveResults.get(run)?.response
  }

  private statusText(response: LiveRunResponse | undefined): string {
    if (response === undefined) return ''
    return String(response.status)
  }

  private contentTypeOf(response: LiveRunResponse | undefined): string {
    return response?.contentType ?? ''
  }

  private truncatedNote(truncated: boolean | undefined): string {
    if (truncated === true) return SnippetRunService.TRUNCATED_NOTE
    return ''
  }

  private draftOf(run: string): RunDraft {
    return this.drafts.get(run) ?? SnippetRunService.EMPTY_DRAFT
  }

  private liveRequest(draft: RunDraft): Record<string, string> {
    const [path, ...query] = draft.path.split(SnippetRunService.QUERY_MARK)
    const request = { method: draft.method, path, query: query.join(SnippetRunService.QUERY_MARK) }
    if (!SnippetRunService.BODY_METHODS.includes(draft.method)) return request
    return { ...request, body: draft.body }
  }

  private outcomeOf(result: LiveRunResult | void | undefined): string {
    return result?.outcome ?? SnippetRunService.OFFLINE_OUTCOME
  }

  private durationOf(result: LiveRunResult | void | undefined): number {
    return result?.durationMs ?? 0
  }

  private messageForOutcome(outcome: string, durationMs: number): string {
    switch (outcome) {
      case SnippetRunService.OK_OUTCOME: return `Live run, ${durationMs} ms`
      case SnippetRunService.OFFLINE_OUTCOME: return SnippetRunService.OFFLINE_MESSAGE
      default: return SnippetRunService.TIMEOUT_MESSAGE
    }
  }

  private rememberLive(run: string, result: LiveRunResult | void | undefined): void {
    if (!result) return
    this.liveResults.set(run, result)
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
    const exchange = this.firstExchange(run)
    this.drafts.set(run, this.draftFrom(exchange, this.readable(this.bodyText(exchange?.request.body))))
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

  private firstExchange(run: string): RecordedExchange | undefined {
    return this.runs.get(run)?.[0]
  }

  private draftFrom(exchange: RecordedExchange | undefined, body: string): RunDraft {
    if (exchange === undefined) return SnippetRunService.EMPTY_DRAFT
    return { method: exchange.request.method, path: exchange.request.path, body }
  }

  private async renderWhileMounted(): Promise<void> {
    if (this.lifecycle.aborted) return
    await this.taskScheduler.render()
  }
}
