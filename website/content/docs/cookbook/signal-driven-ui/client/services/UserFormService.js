/** @import { UserSaved } from '/models/UserSaved.ts' */
@Service
export default class UserFormService {
  /** @type {UsersClient} */
  @Needs
  usersClient;
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  name = '';
  async save() {
    /** @type {UserSaved} */
    const saved = /** @type {UserSaved} */ (await this.usersClient.save(this.name));
    this.applicationEventPublisher.publishEvent(saved);
  }
}
