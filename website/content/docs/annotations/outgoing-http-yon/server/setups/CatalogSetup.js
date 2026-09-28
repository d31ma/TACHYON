@Setup
class CatalogSetup {
  /**
   * @param {object} execution
   * @returns {object}
   */
  static configure(execution) {
    return { http: { catalog: { exchange: async (request) => ({ items: ['anvil'] }) } } };
  }
}
