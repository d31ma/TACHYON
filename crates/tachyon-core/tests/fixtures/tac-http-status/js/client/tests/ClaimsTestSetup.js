@TestSetup
export default class ClaimsTestSetup {
  static configure() {
    return { http: { claims: { exchange: ClaimsTestSetup.exchange } } };
  }

  /** @param {{ path: string }} request */
  static exchange(request) {
    throw ClaimsTestSetup.failures()[request.path];
  }

  static failures() {
    return {
      '/api/claims/built': Behaviors.httpError(422, { title: 'Invalid input' }),
      '/api/claims/plain': Object.assign(new Error('private fake text'), { code: 'HTTP_STATUS', status: 409, body: { reason: 'taken' } }),
      '/api/claims/empty': Behaviors.httpError(404),
      '/api/claims/low': Behaviors.httpError(302, { reason: 'moved' }),
      '/api/claims/unsafe': Object.assign(new Error('private fake text'), { code: 'HTTP_STATUS', status: 409, body: { reason: () => 'taken' } }),
      '/api/claims/other': new Error('private fake text'),
    };
  }
}
