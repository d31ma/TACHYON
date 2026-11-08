import type { HostAnswer } from '/models/HostAnswer.ts'
import type { Problem } from '/models/Problem.ts'
import type { RunRequest } from '/models/RunRequest.ts'
import type { RunResult } from '/models/RunResult.ts'

@Service
export default class SnippetRunService {
  private static readonly OK = 200
  private static readonly BAD_REQUEST = 400
  private static readonly NOT_FOUND = 404
  private static readonly TOO_LARGE = 413
  private static readonly TOO_MANY = 429

  @Needs
  readonly snippetHost!: SnippetHostClient

  async run(runId: string, client: string, run: RunRequest): Promise<RunResult> {
    const request = this.withBody({ method: run.method, path: run.path, query: this.queryValues(run.query) }, run.body)
    const answer = await this.snippetHost.run(runId, client, request) as unknown as HostAnswer
    const value = JSON.parse(answer.body)
    this.throwRefusal(this.refusal(answer.status, value as Problem))
    return value as RunResult
  }

  private queryValues(query: string | undefined): Record<string, string> {
    return Object.fromEntries(new URLSearchParams(query ?? ''))
  }

  private withBody(request: Record<string, BehaviorJsonValue>, body: string | undefined): Record<string, BehaviorJsonValue> {
    if (body === undefined) return request
    return { ...request, body }
  }

  private throwRefusal(refusal: Error | undefined): void {
    if (refusal !== undefined) throw refusal
  }

  private refusal(status: number, problem: Problem): Error | undefined {
    switch (status) {
      case SnippetRunService.OK: return undefined
      case SnippetRunService.BAD_REQUEST: return new InvalidRun(problem)
      case SnippetRunService.NOT_FOUND: return new ExampleNotFound(problem)
      case SnippetRunService.TOO_LARGE: return new RunTooLarge(problem)
      case SnippetRunService.TOO_MANY: return new TooManyRuns(problem)
      default: return new RunnerBusy(problem)
    }
  }
}
