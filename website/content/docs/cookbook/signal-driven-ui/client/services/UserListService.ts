import type { UsersChanged } from '/models/UsersChanged.ts';

@Service
export default class UserListService {
  @Needs
  readonly usersClient!: UsersClient;
  users: BehaviorJsonValue = [];
  @OnStart
  async initialize() { await this.refresh(); }
  @OnEvent
  async usersChanged(changed: UsersChanged) { await this.refreshWhenSaved(changed); }
  private async refresh() { this.users = await this.usersClient.list(); }
  private async refreshWhenSaved(changed: UsersChanged) { if (changed.saved) await this.refresh(); }
}