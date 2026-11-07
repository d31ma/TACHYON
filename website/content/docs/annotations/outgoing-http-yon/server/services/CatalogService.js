@Service
export default class CatalogService {
  /** @type {CatalogClient} */
  @Needs
  catalogClient;
  /** @returns {Promise<unknown>} */
  async products() { return await this.catalogClient.all(1); }
}
