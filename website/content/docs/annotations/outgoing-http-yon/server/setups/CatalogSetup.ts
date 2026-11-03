@Setup
export default class CatalogSetup {
  static configure(execution: BehaviorExecution) {
    return { http: { catalog: { exchange: async () => ({ items: ['anvil'] }) } } };
  }
}
