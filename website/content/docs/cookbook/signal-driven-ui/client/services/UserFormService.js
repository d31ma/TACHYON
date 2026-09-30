/** @import { UsersChanged } from '/models/UsersChanged.ts' */
@Service
export default class UserFormService {
  /** @type {UsersClient} */
  @Needs
  usersClient;
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  /** @type {{ name: string }} */
  form = { name: '' };
  /**
   * @param {Event} _event
   * @param {string} name
   */
  typed(_event, name) { this.form = { ...this.form, name }; }
  async save() {
    await this.usersClient.save(this.form);
    /** @type {UsersChanged} */
    const changed = { saved: true };
    this.applicationEventPublisher.publishEvent(changed);
  }
}