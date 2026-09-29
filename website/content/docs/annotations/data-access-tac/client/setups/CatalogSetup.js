@Setup
export default class CatalogSetup {
  /** @returns {BehaviorConfiguration} */
  static configure() {
    /** @param {BehaviorEntityCommand} command */
    const fetchRows = async (command) => {
      const response = await fetch(`/api/${command.table}`, { headers: { accept: 'application/json' } });
      return response.json();
    };
    return {
      authorizeEntity: (command) => command.entity === 'ProductEntity',
      entities: { publicCatalog: { execute: fetchRows } },
    };
  }
}
