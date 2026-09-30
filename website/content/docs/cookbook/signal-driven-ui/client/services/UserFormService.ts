import type { UsersChanged } from '/models/UsersChanged.ts';

@Service
export default class UserFormService {
  @Needs
  readonly usersClient!: UsersClient;
  applicationEventPublisher!: ApplicationEventPublisher;
  form = { name: '' };
  typed(_event: Event, name: string) { this.form = { ...this.form, name }; }
  async save() {
    await this.usersClient.save(this.form);
    const changed: UsersChanged = { saved: true };
    this.applicationEventPublisher.publishEvent(changed);
  }
}