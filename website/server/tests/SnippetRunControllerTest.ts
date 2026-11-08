import type { RunRequest } from '/models/RunRequest.ts'

@Test(SnippetRunTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class SnippetRunControllerTest {
  @Fake
  snippetRuns!: SnippetRunService;

  async testPostRunPassesTheExampleTheClientAndTheRun(): Promise<void> {
    const calls: unknown[] = [];
    this.snippetRuns = {
      run: async (runId: string, client: string, run: RunRequest) => { calls.push([runId, client, run]); return { outcome: 'ok', durationMs: 3 }; },
    } as unknown as SnippetRunService;
    const controller = this.construct(SnippetRunController);
    const result = await controller.postRun('75236e5355efdd11', '203.0.113.7', { method: 'POST', path: '/echo' });
    Assert.equals('ok', result.outcome, 'the result is the service answer');
    Assert.equals(JSON.stringify(['75236e5355efdd11', '203.0.113.7', { method: 'POST', path: '/echo' }]), JSON.stringify(calls[0]), 'the controller forwards its arguments');
  }
}
