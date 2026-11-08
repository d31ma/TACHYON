@Test(SnippetRunTestSetup, {"partition":"tests"})
@UseProfiles("test")
export default class SnippetRunErrorHandlerTest {
  problem(code: string) {
    return { code, message: `${code} message`, retryAfterSeconds: 5 };
  }

  testInvalidAnswersItsProblem(): void {
    Assert.equals('invalid-request', new SnippetRunErrorHandler().invalid(new InvalidRun(this.problem('invalid-request'))).code);
  }

  testMissingAnswersItsProblem(): void {
    Assert.equals('unknown-example', new SnippetRunErrorHandler().missing(new ExampleNotFound(this.problem('unknown-example'))).code);
  }

  testTooLargeAnswersItsProblem(): void {
    Assert.equals('too-large', new SnippetRunErrorHandler().tooLarge(new RunTooLarge(this.problem('too-large'))).code);
  }

  testTooManyKeepsTheRetryTime(): void {
    Assert.equals(5, new SnippetRunErrorHandler().tooMany(new TooManyRuns(this.problem('rate-limited'))).retryAfterSeconds);
  }

  testBusyAnswersItsProblem(): void {
    Assert.equals('busy', new SnippetRunErrorHandler().busy(new RunnerBusy(this.problem('busy'))).code);
  }
}
