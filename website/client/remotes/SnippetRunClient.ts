import type { LiveRunResult } from '/models/LiveRunResult.ts'

@HTTPClient("/api/snippets")
export default class SnippetRunClient {
  @PostRequest("/{id}/runs")
  async run(@PathVariable("id") id: string, @RequestBody request: BehaviorJsonValue): Promise<LiveRunResult | void> {}
}
