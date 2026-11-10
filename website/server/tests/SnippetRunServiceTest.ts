import type { RunResult } from '/models/RunResult.ts'

@Test(SnippetRunTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class SnippetRunServiceTest {
  @Fake
  snippetHost!: SnippetHostClient;

  answer(status: number, body: object): { sent: Record<string, unknown>[] } {
    const sent: Record<string, unknown>[] = [];
    this.snippetHost = {
      run: async (runId: string, client: string, request: Record<string, unknown>) => {
        sent.push({ runId, client, request });
        return { status, body: JSON.stringify(body) };
      },
    } as unknown as SnippetHostClient;
    return { sent };
  }

  async testRunSendsTheRequestAndReturnsTheResult(): Promise<void> {
    const result: RunResult = { outcome: 'ok', durationMs: 5, response: { status: 200, contentType: 'application/json', body: '{}', truncated: false } };
    const { sent } = this.answer(200, result);
    const service = this.construct(SnippetRunService);
    const answer = await service.run('75236e5355efdd11', '203.0.113.7', { method: 'POST', path: '/echo', query: 'page=2', body: '{"name":"ada"}' });
    Assert.equals(JSON.stringify(result), JSON.stringify(answer), 'the result comes back as the host sent it');
    Assert.equals(JSON.stringify({ runId: '75236e5355efdd11', client: '203.0.113.7', request: { method: 'POST', path: '/echo', query: { page: '2' }, body: '{"name":"ada"}' } }), JSON.stringify(sent[0]), 'the query text becomes values');
  }

  async testARunWithoutABodySendsNoBody(): Promise<void> {
    const { sent } = this.answer(200, { outcome: 'ok', durationMs: 1 });
    const service = this.construct(SnippetRunService);
    await service.run('e89aaa411a01f3b5', '203.0.113.7', { method: 'GET', path: '/items/7' });
    Assert.equals(JSON.stringify({ method: 'GET', path: '/items/7', query: {} }), JSON.stringify(sent[0].request), 'no body key');
  }

  async refusalFor(status: number): Promise<string> {
    this.answer(status, { code: 'refused', message: 'refused' });
    const service = this.construct(SnippetRunService);
    return await service.run('75236e5355efdd11', '203.0.113.7', { method: 'GET', path: '/' }).then(() => 'none', (error: Error) => error.constructor.name);
  }

  async testA400BecomesInvalidRun(): Promise<void> {
    Assert.equals('InvalidRun', await this.refusalFor(400));
  }

  async testA404BecomesExampleNotFound(): Promise<void> {
    Assert.equals('ExampleNotFound', await this.refusalFor(404));
  }

  async testA413BecomesRunTooLarge(): Promise<void> {
    Assert.equals('RunTooLarge', await this.refusalFor(413));
  }

  async testA429BecomesTooManyRuns(): Promise<void> {
    Assert.equals('TooManyRuns', await this.refusalFor(429));
  }

  async testAnyOtherStatusBecomesRunnerBusy(): Promise<void> {
    Assert.equals('RunnerBusy', await this.refusalFor(503));
  }
}
