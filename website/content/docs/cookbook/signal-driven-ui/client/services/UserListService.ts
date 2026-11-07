import type { UserSaved } from '/models/UserSaved.ts';

@Service
export default class UserListService {
  names: string[] = [];
  @OnEvent
  userSaved(saved: UserSaved) { this.names = [...this.names, saved.name]; }
}
