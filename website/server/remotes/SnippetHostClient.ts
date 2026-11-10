@HTTPClient
export default class SnippetHostClient {
  @PostRequest("/runs/{runId}")
  async run(@PathVariable("runId") runId: string, @RequestHeader("X-Snippet-Client") client: string, @RequestBody request: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}
}
