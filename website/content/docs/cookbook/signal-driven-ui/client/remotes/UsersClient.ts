import type { UserSaved } from '/models/UserSaved.ts';

@HTTPClient
export default class UsersClient {
  @PostRequest("/users")
  async save(@RequestBody name: string): Promise<UserSaved | void> {}
}
