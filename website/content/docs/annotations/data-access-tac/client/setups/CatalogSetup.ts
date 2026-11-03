@Setup
export default class CatalogSetup {
  static configure() {
    const fetchRows = async (command: any) => {
      const response = await fetch(`/api/${command.table}`, { headers: { accept: 'application/json' } });
      return response.json();
    };
    return {
      authorizeEntity: (command: any) => command.entity === 'ProductEntity',
      entities: { publicCatalog: { execute: fetchRows } },
    };
  }
}
