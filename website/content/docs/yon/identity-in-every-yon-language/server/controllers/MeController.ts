import type { Identity } from '/models/Identity.ts';

@RequestRoute("/me")
@SignedIn
export class MeController {
  @GetRoute
  getMe(@FromIdentity caller: Identity): string {
    return caller.user;
  }
}
