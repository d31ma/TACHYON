@Service
export default class CatalogService {
  /** @type {CatalogClient} */
  @Needs
  catalogClient;
  count = 0;

  async load() {
    const answer = await this.catalogClient.all();
    this.count = this._countItems(answer);
  }

  /**
   * @param {{ items: string[] } | void} answer
   * @returns {number}
   */
  _countItems(answer) {
    return answer ? answer.items.length : 0;
  }
}
