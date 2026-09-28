import type { Identity } from '/models/Identity.ts';

@IdentityProvider
export class AppIdentityProvider {
  private static readonly SESSIONS = new Map<string, Identity>([
    ['ada-token', { user: 'ada', roles: ['Admin'] }],
    ['bob-token', { user: 'bob', roles: [] }],
  ]);

  identify(request: Request): Identity | null {
    return this.#find(String(request.headers.get('authorization')));
  }

  #find(token: string): Identity | null {
    return AppIdentityProvider.SESSIONS.get(token) ?? null;
  }
}
