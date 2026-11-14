import type { Identity } from '/models/Identity.ts';

@IdentityProvider
export class AppIdentityProvider {
  private static readonly SESSIONS = new Map<string, Identity>([
    ['ada-token', { user: 'ada', roles: ['Admin'] }],
    ['bob-token', { user: 'bob', roles: [] }],
  ]);

  private static readonly BROKEN_TOKEN = 'broken-token';

  identify(request: Request): Identity | null {
    const token = String(request.headers.get('authorization'));
    this.refuseBroken(token);
    return this.find(token);
  }

  private refuseBroken(token: string): void {
    if (token === AppIdentityProvider.BROKEN_TOKEN) throw new Error('the session store is down');
  }

  private find(token: string): Identity | null {
    return AppIdentityProvider.SESSIONS.get(token) ?? null;
  }
}
