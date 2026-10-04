/** @import { UserSaved } from '/models/UserSaved.ts' */
@Service
export default class UserListService {
  /** @type {string[]} */
  names = [];
  /** @param {UserSaved} saved */
  @OnEvent
  userSaved(saved) { this.names = [...this.names, saved.name]; }
}
