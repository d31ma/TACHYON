@Setup
export default class CatalogSetup {
  /**
   * @param {BehaviorExecution} execution
   * @returns {BehaviorConfiguration}
   */
  static configure(execution) {
    return { http: { catalog: { exchange: async () => ({ items: ['anvil'] }) } } };
  }
}
