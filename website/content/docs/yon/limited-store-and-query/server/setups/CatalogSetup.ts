@Setup
class CatalogSetup {
  static configure() {
    const rows = [{ id: 1, status: 'active' }, { id: 2, status: 'retired' }];
    const findRows = (command: any) => {
      const query = command.query;
      const condition = query.where[0];
      const keepUnequalRows = condition.operator === 'ne';
      const matchingRows = rows.filter((row: any) => {
        const isEqual = row[condition.field] === condition.value;
        if (keepUnequalRows) return !isEqual;
        return isEqual;
      });
      if (query.kind === 'count') return matchingRows.length;
      return matchingRows.slice(0, query.page.size);
    };
    return {
      authorizeEntity: (command: any) => command.entity === 'ProductEntity',
      entities: { catalog: { execute: findRows } },
    };
  }
}
