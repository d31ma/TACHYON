import type { RunRequest } from '/models/RunRequest.ts'
import type { RunResult } from '/models/RunResult.ts'

@RequestRoute("/api/snippets")
@Public
export class SnippetRunController {
  @Needs
  readonly snippetRuns!: SnippetRunService

  @PostRoute("/{id}/runs")
  async postRun(@FromPath id: string, @FromHeader('x-real-ip') client: string, @FromBody run: RunRequest): Promise<RunResult> {
    return await this.snippetRuns.run(id, client, run)
  }
}
