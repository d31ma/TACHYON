@Service
export default class ClaimService {
  /** @type {ClaimsClient} */
  @Needs
  claimsClient;

  /**
   * @param {string} id
   * @returns {Promise<BehaviorJsonValue>}
   */
  async claim(id) {
    return this.claimsClient.claim(id);
  }
}
