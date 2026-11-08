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
    Assert.equals('The recorded output could not be loaded. Try again.', service.status('abc'), 'the status says what happened')
  }

  async testAnEmptyAnswerCountsAsAFailedLoad() {
    const service = new SnippetRunService()
    this.mounted(service, async () => undefined)
    await service.toggle(null, 'abc')
    Assert.equals('The recorded output could not be loaded. Try again.', service.status('abc'), 'an empty answer is a failure')
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
}
