import type { UserSaved } from '/models/UserSaved.ts';

@Service
export default class UserFormService {
  @Needs
  readonly usersClient!: UsersClient;
  applicationEventPublisher!: ApplicationEventPublisher;
  name = '';
  async save() {
    const saved: UserSaved = await this.usersClient.save(this.name) as UserSaved;
    this.applicationEventPublisher.publishEvent(saved);
  }
}
