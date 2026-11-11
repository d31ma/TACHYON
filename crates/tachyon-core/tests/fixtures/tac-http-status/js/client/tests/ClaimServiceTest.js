@Test(ClaimsTestSetup, {"partition":"tac-http-status"})
export default class ClaimServiceTest {
  /** @type {ClaimsClient} */
  @Fake
  claimsClient;

  service() {
    this.claimsClient = Object.assign(new ClaimsClient(), { behaviors: this.behaviors });
    return this.construct(ClaimService);
  }

  async testABuiltErrorKeepsItsStatusAndBody() {
    const error = await Assert.throws(() => this.service().claim('built'));
    Assert.equals('HTTP_STATUS', error.code);
    Assert.equals(422, Behaviors.errorStatus(error));
    Assert.equals({ title: 'Invalid input' }, Behaviors.errorBody(error));
  }

  async testAPlainErrorWithTheCodeLosesItsOwnMessage() {
    const error = await Assert.throws(() => this.service().claim('plain'));
    Assert.equals('BehaviorError', error.name);
    Assert.equals('The HTTP request failed.', error.message);
    Assert.equals(409, error.status);
    Assert.equals({ reason: 'taken' }, error.body);
  }

  async testAnErrorWithoutABodyHasNoBody() {
    const error = await Assert.throws(() => this.service().claim('empty'));
    Assert.equals(404, Behaviors.errorStatus(error));
    Assert.equals(null, Behaviors.errorBody(error));
  }

  async testAStatusOutsideTheErrorRangeIsAnAdapterFailure() {
    const error = await Assert.throws(() => this.service().claim('low'));
    Assert.equals('adapter', error.code);
    Assert.equals(0, Behaviors.errorStatus(error));
  }

  async testABodyThatIsNotJsonIsAnAdapterFailure() {
    const error = await Assert.throws(() => this.service().claim('unsafe'));
    Assert.equals('adapter', error.code);
    Assert.equals(null, Behaviors.errorBody(error));
  }

  async testAnotherAdapterErrorStaysAnAdapterFailure() {
    const error = await Assert.throws(() => this.service().claim('other'));
    Assert.equals('adapter', error.code);
    Assert.equals('Behavior adapter operation failed.', error.message);
  }

  testValuesThatAreNotHttpStatusErrorsHaveNoStatusAndNoBody() {
    const lookalike = Object.assign(new Error('plain'), { code: 'HTTP_STATUS', status: 409, body: { reason: 'taken' } });
    Assert.equals(0, Behaviors.errorStatus(lookalike));
    Assert.equals(0, Behaviors.errorStatus(null));
    Assert.equals(null, Behaviors.errorBody(lookalike));
    Assert.equals(null, Behaviors.errorBody('text'));
  }
}
