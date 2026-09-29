/** @import { UsersChanged } from '/models/UsersChanged.ts' */
@Service
export default class UserListService {
  /** @type {UsersClient} */
  @Needs
  usersClient;
  /** @type {{ name: string }[]} */
  users = [];
  @OnStart
  async initialize() { await this._refresh(); }
  /** @param {UsersChanged} changed */
  @OnEvent
  async usersChanged(changed) { await this.#refreshWhenSaved(changed); }
  async _refresh() { this.users = (await this.usersClient.list()) ?? []; }
  /** @param {UsersChanged} changed */
  async #refreshWhenSaved(changed) { if (changed.saved) await this._refresh(); }
}