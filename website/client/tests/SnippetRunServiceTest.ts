@Test(SiteTestSetup, {"partition":"tests"})
export default class SnippetRunServiceTest {
  exchange(contentType: string, body: string): RecordedExchange {
    return {
      request: { method: 'POST', path: '/echo', headers: { authorization: 'ada-token' }, body: '{"name":"snippet"}' },
      response: { status: 200, contentType, body },
    }
  }

  mounted(service: SnippetRunService, answer: () => Promise<RecordedRun | void>): { lifetime: AbortController, asked: string[] } {
    const asked: string[] = []
    service.recordedRunClient = { run: async (file: string) => { asked.push(file); return answer() } } as RecordedRunClient
    service.snippetRunClient = { run: async () => { throw Error('offline') } } as unknown as SnippetRunClient
    service.taskScheduler = { render: async () => {} }
    const lifetime = new AbortController()
    service.lifecycle = lifetime.signal
    return { lifetime, asked }
  }

  async testRunLoadsTheRecordedFileAndShowsItsExchanges() {
    const service = new SnippetRunService()
    const exchange = this.exchange('application/json', '{"name":"snippet"}')
    const { asked } = this.mounted(service, async () => ({ schema: 1, exchanges: [exchange] }))
    Assert.equals('Run', service.label('abc'), 'a run starts hidden')
    await service.toggle(null, 'abc')
    Assert.equals('abc.json', asked[0], 'it asks for the run file by id')
    Assert.isTrue(service.isShown('abc'), 'the run is shown')
    Assert.equals('Hide output', service.label('abc'), 'the button offers to hide it')
    Assert.equals(1, service.exchanges('abc').length, 'with its exchange')
    Assert.equals('', service.status('abc'), 'and no message')
  }

  async testRunAgainHidesTheOutputWithoutAskingAgain() {
    const service = new SnippetRunService()
    const { asked } = this.mounted(service, async () => ({ schema: 1, exchanges: [] }))
    await service.toggle(null, 'abc')
    await service.toggle(null, 'abc')
    Assert.isTrue(!service.isShown('abc'), 'the second click hides the run')
    Assert.equals(1, asked.length, 'without asking the server again')
  }

  async testAFailedLoadSaysSoAndShowsNothing() {
    const service = new SnippetRunService()
    this.mounted(service, async () => { throw Error('offline') })
    await service.toggle(null, 'abc')
    Assert.isTrue(!service.isShown('abc'), 'nothing is shown')
    Assert.equals('No recorded output in this build.', service.status('abc'), 'the status says what happened')
  }

  async testAnEmptyAnswerCountsAsAFailedLoad() {
    const service = new SnippetRunService()
    this.mounted(service, async () => undefined)
    await service.toggle(null, 'abc')
    Assert.equals('No recorded output in this build.', service.status('abc'), 'an empty answer is a failure')
  }

  async testAnUnknownRunHasNoExchanges() {
    const service = new SnippetRunService()
    this.mounted(service, async () => ({ schema: 1, exchanges: [] }))
    Assert.equals(0, service.exchanges('missing').length, 'nothing loaded means no exchanges')
  }

  testRequestTextListsHeadersThenTheReadableBody() {
    const service = new SnippetRunService()
    const text = service.requestText(this.exchange('application/json', '{}'))
    Assert.equals('authorization: ada-token\n{\n  "name": "snippet"\n}', text, 'headers come first, then the body')
  }

  testAJsonResponseIsIndented() {
    const service = new SnippetRunService()
    Assert.equals('{\n  "level": 4\n}', service.responseText(this.exchange('application/json; charset=utf-8', '{"level":4}')), 'JSON is indented')
  }

  testOtherResponsesStayAsTheyAre() {
    const service = new SnippetRunService()
    Assert.equals('{"level":4}', service.responseText(this.exchange('text/plain', '{"level":4}')), 'plain text is unchanged')
    Assert.equals('<p>hi</p>', service.responseText(this.exchange('text/html', '<p>hi</p>')), 'HTML is unchanged')
  }

  async testAnUnmountedServiceDoesNotRender() {
    const service = new SnippetRunService()
    let renders = 0
    const { lifetime } = this.mounted(service, async () => ({ schema: 1, exchanges: [] }))
    service.taskScheduler = { render: async () => { renders += 1 } }
    lifetime.abort()
    await service.toggle(null, 'abc')
    Assert.equals(0, renders, 'an unmounted snippet never renders')
  }

  live(service: SnippetRunService, answer: () => Promise<LiveRunResult | void>): unknown[][] {
    const sent: unknown[][] = []
    service.snippetRunClient = { run: async (id: string, request: unknown) => { sent.push([id, request]); return answer() } } as unknown as SnippetRunClient
    return sent
  }

  async loaded(): Promise<SnippetRunService> {
    const service = new SnippetRunService()
    this.mounted(service, async () => ({ schema: 1, exchanges: [this.exchange('application/json', '{"name":"snippet"}')] }))
    await service.toggle(null, 'abc')
    return service
  }

  async testTheDraftStartsAsTheRecordedRequest() {
    const service = await this.loaded()
    Assert.equals('POST', service.draftMethod('abc'), 'the recorded method')
    Assert.equals('/echo', service.draftPath('abc'), 'the recorded path')
    Assert.equals('{\n  "name": "snippet"\n}', service.draftBody('abc'), 'the recorded body, indented')
    Assert.isTrue(service.takesBody('abc'), 'a POST takes a body')
  }

  async testEditsChangeTheDraft() {
    const service = await this.loaded()
    service.editPath(null, 'abc', '/echo?x=1')
    service.editBody(null, 'abc', '{"name":"ada"}')
    Assert.equals('/echo?x=1', service.draftPath('abc'), 'the new path')
    Assert.equals('{"name":"ada"}', service.draftBody('abc'), 'the new body')
  }

  testAnUnknownRunHasAnEmptyGetDraft() {
    const service = new SnippetRunService()
    Assert.equals('GET', service.draftMethod('none'), 'an empty draft is a GET')
    Assert.isTrue(!service.takesBody('none'), 'a GET takes no body')
  }

  async testRunLiveSendsTheDraftAndShowsTheAnswer() {
    const service = await this.loaded()
    const sent = this.live(service, async () => ({ outcome: 'ok', durationMs: 12, response: { status: 200, contentType: 'application/json', body: '{"name":"ada"}', truncated: false } }))
    service.editBody(null, 'abc', '{"name":"ada"}')
    await service.runLive(null, 'abc')
    Assert.equals(JSON.stringify(['abc', { method: 'POST', path: '/echo', query: '', body: '{"name":"ada"}' }]), JSON.stringify(sent[0]), 'the draft is sent')
    Assert.isTrue(service.hasLiveAnswer('abc'), 'the answer is shown')
    Assert.equals('200', service.liveStatus('abc'), 'with its status')
    Assert.equals('application/json', service.liveContentType('abc'), 'its content type')
    Assert.equals('{\n  "name": "ada"\n}', service.liveBody('abc'), 'and its indented body')
    Assert.equals('Live run, 12 ms', service.liveMessage('abc'), 'the status gives the time')
  }

  async testAGetRunLiveSendsNoBody() {
    const service = new SnippetRunService()
    this.mounted(service, async () => ({ schema: 1, exchanges: [{ request: { method: 'GET', path: '/items/7', headers: {} }, response: { status: 200, contentType: 'application/json', body: '{}' } }] }))
    await service.toggle(null, 'abc')
    const sent = this.live(service, async () => ({ outcome: 'ok', durationMs: 1 }))
    await service.runLive(null, 'abc')
    Assert.equals(JSON.stringify({ method: 'GET', path: '/items/7', query: '' }), JSON.stringify(sent[0][1]), 'no body key')
  }

  async testAnOfflineRunnerKeepsTheRecordedOutput() {
    const service = await this.loaded()
    await service.runLive(null, 'abc')
    Assert.equals('The live runner did not answer. The output above is the recorded run.', service.liveMessage('abc'), 'the status says so')
    Assert.isTrue(!service.hasLiveAnswer('abc'), 'no live answer is shown')
    Assert.isTrue(service.isShown('abc'), 'the recorded output stays')
  }

  async testATimeoutSaysTheExampleWasStopped() {
    const service = await this.loaded()
    this.live(service, async () => ({ outcome: 'timeout', durationMs: 10000 }))
    await service.runLive(null, 'abc')
    Assert.equals('The example took too long and was stopped.', service.liveMessage('abc'), 'the status says so')
    Assert.equals('', service.liveStatus('abc'), 'and shows no status')
  }

  async testACutAnswerSaysSo() {
    const service = await this.loaded()
    this.live(service, async () => ({ outcome: 'ok', durationMs: 3, response: { status: 200, contentType: 'text/plain', body: 'xx', truncated: true } }))
    await service.runLive(null, 'abc')
    Assert.equals('xx\n[The answer was cut at 64 KiB.]', service.liveBody('abc'), 'the note follows the body')
  }

  testNoLiveRunMeansNoLiveText() {
    const service = new SnippetRunService()
    Assert.equals('', service.liveMessage('abc'), 'no message')
    Assert.equals('', service.liveContentType('abc'), 'no content type')
    Assert.equals('', service.liveBody('abc'), 'no body')
  }

  async testAQueryInThePathIsSentAsTheQuery() {
    const service = await this.loaded()
    const sent = this.live(service, async () => ({ outcome: 'ok', durationMs: 1 }))
    service.editPath(null, 'abc', '/echo?page=12&sort=name')
    await service.runLive(null, 'abc')
    Assert.equals(JSON.stringify({ method: 'POST', path: '/echo', query: 'page=12&sort=name', body: '{\n  "name": "snippet"\n}' }), JSON.stringify(sent[0][1]), 'path and query are apart')
  }
}
